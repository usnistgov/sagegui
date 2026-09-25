// Written by Benjamin A. Neely (NIST) on 2026-09-23.
//! On-disk cache of the built Sage peptide database (`IndexedDatabase`).
//!
//! Building the database is the slowest step of a run: Sage reads the FASTA,
//! digests it, generates every theoretical fragment, then sorts and buckets
//! them. For a human proteome that is about two minutes. The result depends
//! only on the FASTA content, the database parameters and the Sage code, so a
//! later run with the same three can load it instead of building it again.
//!
//! **Key.** A 128-bit xxHash3 over, in order and length-prefixed:
//! [`FORMAT_VERSION`], the strategy byte, `version::SAGE_COMMIT`, the SageGUI
//! version, the canonical JSON of the resolved `Parameters` (without the FASTA
//! path and the prefilter settings), and a hash of the FASTA bytes. Search
//! settings that do not change the database (tolerances, charges, quant, the
//! spectrum files) are not in the key, so changing only those is a cache hit.
//! Prefiltering builds a database that depends on the spectra, so it is never
//! cached.
//!
//! **File.** `sage-index-<key>.bin`: a fixed header (magic, strategy, format
//! version, key, payload length, payload checksum) and a little-endian binary
//! payload. Floats are stored as their exact bits, so a loaded database is
//! bit-identical to the one that was stored. JSON would round `f32` values
//! through decimal text, and the masses feed sorts, equality tests and binary
//! searches. Protein accessions and peptide sequences are stored once each and
//! shared again through `Arc` on load, as Sage shares them when it builds.
//!
//! **Failure is never an error.** `load` returns `None` for a missing, short,
//! corrupt, wrong-version or wrong-key file, and for a panic inside the reader.
//! The caller then builds the database as usual. `store` logs and swallows
//! every error, because the search has already succeeded. On Windows a panic in
//! the search thread shows up as a failed run (`windows_subsystem`), so the
//! reader catches its own panics instead of relying on the caller.
//!
//! **Order.** The cache holds one valid build. Stock Sage already varies the
//! order of `potential_mods` and of equal-mass fragments from run to run. That
//! order does not change which PSMs are found or their scores. We do not
//! reorder anything here, because a cache-only reordering would make a cached
//! run differ from a fresh one.

/// Shows the cache in the UI and lets a search use it. Off since 2026-09-25:
/// the code is complete and tested, but the maintainer is not ready to ship
/// it. While this is false, the checkbox and the Run / Info group are hidden,
/// and a saved `reuse_cached_index: true` has no effect. See NOTES, Database
/// cache, for what else to restore when this goes back to true.
pub const ENABLED: bool = false;

use std::collections::HashMap;
use std::fs::{self, File};
use std::hash::Hasher as _;
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use sage_core::database::{IndexedDatabase, Parameters, PeptideIx, Theoretical};
use sage_core::enzyme::Position;
use sage_core::ion_series::Kind;
use sage_core::modification::ModificationSpecificity;
use sage_core::peptide::Peptide;
use twox_hash::{XxHash3_128, XxHash3_64};

/// Bump on any change to the file layout, or to a vendored Sage patch that
/// changes what the database contains. A mismatch is a miss, never an error.
pub const FORMAT_VERSION: u32 = 1;

/// The payload is a full `IndexedDatabase`. A different strategy (for example
/// peptides only, rebuilt on load) would use another value.
const STRATEGY_FULL_INDEX: u8 = 0;

const MAGIC: &[u8; 5] = b"SGIDX";
/// magic 5 + strategy 1 + format 4 + key 16 + payload length 8 + checksum 8.
const HEADER_LEN: u64 = 42;

/// The cache folder may hold this much. Storing a new entry first deletes the
/// least recently used entries until the new one fits.
pub const MAX_CACHE_BYTES: u64 = 20 << 30;
/// An entry estimated larger than this is not written at all.
pub const MAX_ENTRY_BYTES: u64 = 12 << 30;

const FILE_PREFIX: &str = "sage-index-";
const FILE_SUFFIX: &str = ".bin";
const TEMP_PREFIX: &str = ".tmp-";

/// Read and write in blocks of this many array elements.
const BLOCK: usize = 1 << 16;

/// One cache entry: where it lives and the key it must carry.
pub struct IndexCache {
    dir: PathBuf,
    path: PathBuf,
    key: [u8; 16],
}

/// What `store` did, for the run bar.
pub enum StoreOutcome {
    Stored { bytes: u64 },
    TooLarge { estimate: u64 },
    Failed(String),
}

/// Totals for the cache panel on Run / Info.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CacheStats {
    pub files: usize,
    pub bytes: u64,
}

/// The folder that holds the cache files, or `None` when the OS gives no home
/// or cache folder. Per OS:
/// - macOS: `~/Library/Caches/gov.nist.sagegui/index-cache`
/// - Windows: `%LOCALAPPDATA%\SageGUI\index-cache`
/// - Linux and others: `$XDG_CACHE_HOME/sagegui/index-cache`, else
///   `~/.cache/sagegui/index-cache`
pub fn cache_dir() -> Option<PathBuf> {
    let non_empty = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());

    #[cfg(target_os = "macos")]
    let base = non_empty("HOME").map(|h| {
        PathBuf::from(h)
            .join("Library")
            .join("Caches")
            .join("gov.nist.sagegui")
    });

    #[cfg(target_os = "windows")]
    let base = non_empty("LOCALAPPDATA").map(|d| PathBuf::from(d).join("SageGUI"));

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let base = non_empty("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| non_empty("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .map(|p| p.join("sagegui"));

    base.map(|b| b.join("index-cache"))
}

impl IndexCache {
    /// The cache entry for this database, or `None` when caching does not
    /// apply: the option is off, prefiltering is on, the FASTA is not a local
    /// file, or the FASTA cannot be read. Hashes the FASTA content, which takes
    /// well under a second for a proteome-sized file.
    pub fn for_parameters(params: &Parameters, enabled: bool) -> Option<Self> {
        if !enabled || params.prefilter || params.fasta.contains("://") {
            return None;
        }
        let dir = cache_dir()?;
        match Self::in_dir(params, dir) {
            Ok(cache) => Some(cache),
            Err(e) => {
                log::info!("database cache not used: {e}");
                None
            }
        }
    }

    fn in_dir(params: &Parameters, dir: PathBuf) -> io::Result<Self> {
        let fasta_hash = hash_file(Path::new(&params.fasta))?;
        let key = cache_key(params, fasta_hash)?;
        let path = dir.join(format!(
            "{FILE_PREFIX}{:032x}{FILE_SUFFIX}",
            u128::from_le_bytes(key)
        ));
        Ok(Self { dir, path, key })
    }

    /// The cached database, or `None` on a miss or any problem with the file.
    /// A hit marks the file as used now, for least-recently-used eviction.
    pub fn load(&self) -> Option<IndexedDatabase> {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            read_index_file(&self.path, &self.key)
        }));
        match result {
            Ok(Ok(db)) => {
                if let Ok(file) = File::options().write(true).open(&self.path) {
                    let _ = file.set_modified(SystemTime::now());
                }
                Some(db)
            }
            Ok(Err(e)) if e.kind() == io::ErrorKind::NotFound => None,
            Ok(Err(e)) => {
                log::info!(
                    "database cache file not used ({}): {e}",
                    self.path.display()
                );
                None
            }
            Err(_) => {
                log::warn!(
                    "database cache reader panicked on {}; building instead",
                    self.path.display()
                );
                None
            }
        }
    }

    /// Write `db` to the cache. Best effort: nothing here can fail the search.
    pub fn store(&self, db: &IndexedDatabase) -> StoreOutcome {
        self.store_with_limits(db, MAX_ENTRY_BYTES, MAX_CACHE_BYTES)
    }

    fn store_with_limits(
        &self,
        db: &IndexedDatabase,
        max_entry: u64,
        max_total: u64,
    ) -> StoreOutcome {
        let estimate = estimate_size(db);
        if estimate > max_entry {
            return StoreOutcome::TooLarge { estimate };
        }
        let result = (|| -> io::Result<u64> {
            fs::create_dir_all(&self.dir)?;
            evict(&self.dir, max_total.saturating_sub(estimate), &self.path)?;
            let tmp = self.dir.join(format!(
                "{TEMP_PREFIX}{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ));
            let written = write_index_file(&tmp, &self.key, db).and_then(|bytes| {
                fs::rename(&tmp, &self.path)?;
                Ok(bytes)
            });
            if written.is_err() {
                let _ = fs::remove_file(&tmp);
            }
            written
        })();
        match result {
            Ok(bytes) => {
                log::info!(
                    "cached the peptide database ({bytes} bytes) at {}",
                    self.path.display()
                );
                StoreOutcome::Stored { bytes }
            }
            Err(e) => {
                log::info!("database cache not written: {e}");
                StoreOutcome::Failed(e.to_string())
            }
        }
    }
}

/// A size for the interface: GB from 1 GB up, else MB. Decimal units, as in
/// the Results status line.
pub fn size_text(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1e9)
    } else {
        format!("{:.0} MB", bytes as f64 / 1e6)
    }
}

/// File count and total size of the cache entries. Reads the folder, so call
/// it on demand, never every frame.
pub fn stats() -> CacheStats {
    cache_dir().map(|d| stats_in(&d)).unwrap_or_default()
}

fn stats_in(dir: &Path) -> CacheStats {
    entries(dir)
        .into_iter()
        .fold(CacheStats::default(), |acc, e| CacheStats {
            files: acc.files + 1,
            bytes: acc.bytes + e.size,
        })
}

/// Delete every cache entry and every leftover temporary file. Returns the
/// bytes freed. Deletes only files whose names this module creates.
pub fn clear() -> io::Result<u64> {
    match cache_dir() {
        Some(dir) => clear_in(&dir),
        None => Ok(0),
    }
}

fn clear_in(dir: &Path) -> io::Result<u64> {
    let mut freed = 0;
    let listing = match fs::read_dir(dir) {
        Ok(l) => l,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };
    for item in listing.flatten() {
        let name = item.file_name().to_string_lossy().to_string();
        let ours = is_entry_name(&name) || name.starts_with(TEMP_PREFIX);
        if !ours {
            continue;
        }
        let size = item.metadata().map(|m| m.len()).unwrap_or(0);
        if fs::remove_file(item.path()).is_ok() {
            freed += size;
        }
    }
    Ok(freed)
}

// ─── Key ──────────────────────────────────────────────────────────────────────

/// The resolved database parameters as canonical JSON bytes, without the
/// FASTA path (its content is hashed separately) and without the prefilter
/// settings (a prefiltered database is never cached). `serde_json` here is
/// built without `preserve_order`, so objects are sorted maps and the bytes do
/// not depend on `HashMap` iteration order.
fn canonical_params(params: &Parameters) -> io::Result<Vec<u8>> {
    let mut value = serde_json::to_value(params).map_err(io::Error::other)?;
    if let Some(obj) = value.as_object_mut() {
        for key in [
            "fasta",
            "prefilter",
            "prefilter_chunk_size",
            "prefilter_low_memory",
        ] {
            obj.remove(key);
        }
    }
    serde_json::to_vec(&value).map_err(io::Error::other)
}

fn cache_key(params: &Parameters, fasta_hash: u128) -> io::Result<[u8; 16]> {
    let canonical = canonical_params(params)?;
    let mut h = XxHash3_128::new();
    let mut field = |bytes: &[u8]| {
        h.write(&(bytes.len() as u64).to_le_bytes());
        h.write(bytes);
    };
    field(&FORMAT_VERSION.to_le_bytes());
    field(&[STRATEGY_FULL_INDEX]);
    field(crate::version::SAGE_COMMIT.as_bytes());
    field(env!("CARGO_PKG_VERSION").as_bytes());
    field(&canonical);
    field(&fasta_hash.to_le_bytes());
    Ok(h.finish_128().to_le_bytes())
}

fn hash_file(path: &Path) -> io::Result<u128> {
    let mut file = File::open(path)?;
    let mut h = XxHash3_128::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.write(&buf[..n]);
    }
    Ok(h.finish_128())
}

// ─── Folder housekeeping ──────────────────────────────────────────────────────

struct Entry {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

fn is_entry_name(name: &str) -> bool {
    name.starts_with(FILE_PREFIX) && name.ends_with(FILE_SUFFIX)
}

fn entries(dir: &Path) -> Vec<Entry> {
    let Ok(listing) = fs::read_dir(dir) else {
        return Vec::new();
    };
    listing
        .flatten()
        .filter(|item| is_entry_name(&item.file_name().to_string_lossy()))
        .filter_map(|item| {
            let meta = item.metadata().ok()?;
            meta.is_file().then(|| Entry {
                path: item.path(),
                size: meta.len(),
                modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            })
        })
        .collect()
}

/// Delete the least recently used entries until the others total at most
/// `budget` bytes. `keep` is the entry about to be written; it is replaced,
/// not counted. Also deletes temporary files older than a day, which a crash
/// during a write leaves behind.
fn evict(dir: &Path, budget: u64, keep: &Path) -> io::Result<()> {
    if let Ok(listing) = fs::read_dir(dir) {
        let day = std::time::Duration::from_secs(24 * 3600);
        for item in listing.flatten() {
            if !item.file_name().to_string_lossy().starts_with(TEMP_PREFIX) {
                continue;
            }
            let old = item
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age > day);
            if old {
                let _ = fs::remove_file(item.path());
            }
        }
    }

    let mut list: Vec<Entry> = entries(dir)
        .into_iter()
        .filter(|e| e.path != keep)
        .collect();
    list.sort_by_key(|e| e.modified);
    let mut total: u64 = list.iter().map(|e| e.size).sum();
    for e in list {
        if total <= budget {
            break;
        }
        fs::remove_file(&e.path)?;
        total -= e.size;
    }
    Ok(())
}

// ─── Codec ────────────────────────────────────────────────────────────────────

/// An upper bound on the file size, used for the size limits before writing.
fn estimate_size(db: &IndexedDatabase) -> u64 {
    let peptides: u64 = db
        .peptides
        .iter()
        .map(|p| {
            // Fixed fields, then modifications, protein indices, and the
            // sequence and accessions as if none were shared.
            32 + 4 * p.modifications.len() as u64
                + 4 * p.proteins.len() as u64
                + 4
                + p.sequence.len() as u64
                + p.proteins.iter().map(|a| 4 + a.len() as u64).sum::<u64>()
        })
        .sum();
    HEADER_LEN
        + 256
        + db.decoy_tag.len() as u64
        + db.ion_kinds.len() as u64
        + 4 * db.min_value.len() as u64
        + 16 * db.potential_mods.len() as u64
        + peptides
        + 8 * db.fragments.len() as u64
}

fn kind_code(k: Kind) -> u8 {
    match k {
        Kind::A => 0,
        Kind::B => 1,
        Kind::C => 2,
        Kind::X => 3,
        Kind::Y => 4,
        Kind::Z => 5,
    }
}

fn kind_from(code: u8) -> io::Result<Kind> {
    Ok(match code {
        0 => Kind::A,
        1 => Kind::B,
        2 => Kind::C,
        3 => Kind::X,
        4 => Kind::Y,
        5 => Kind::Z,
        _ => return Err(bad("ion kind")),
    })
}

fn position_code(p: Position) -> u8 {
    match p {
        Position::Nterm => 0,
        Position::Cterm => 1,
        Position::Full => 2,
        Position::Internal => 3,
    }
}

fn position_from(code: u8) -> io::Result<Position> {
    Ok(match code {
        0 => Position::Nterm,
        1 => Position::Cterm,
        2 => Position::Full,
        3 => Position::Internal,
        _ => return Err(bad("peptide position")),
    })
}

fn bad(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("bad {what}"))
}

/// A writer that counts and checksums everything written through it.
struct HashingWriter<W: Write> {
    inner: W,
    hasher: XxHash3_64,
    len: u64,
}

impl<W: Write> HashingWriter<W> {
    fn u8(&mut self, v: u8) -> io::Result<()> {
        self.write_all(&[v])
    }
    fn u32(&mut self, v: u32) -> io::Result<()> {
        self.write_all(&v.to_le_bytes())
    }
    fn u64(&mut self, v: u64) -> io::Result<()> {
        self.write_all(&v.to_le_bytes())
    }
    fn f32(&mut self, v: f32) -> io::Result<()> {
        self.write_all(&v.to_bits().to_le_bytes())
    }
    fn len32(&mut self, n: usize) -> io::Result<()> {
        self.u32(u32::try_from(n).map_err(|_| bad("length"))?)
    }
    fn bytes(&mut self, b: &[u8]) -> io::Result<()> {
        self.len32(b.len())?;
        self.write_all(b)
    }
    fn opt_f32(&mut self, v: Option<f32>) -> io::Result<()> {
        match v {
            Some(x) => {
                self.u8(1)?;
                self.f32(x)
            }
            None => self.u8(0),
        }
    }
    fn opt_u8(&mut self, v: Option<u8>) -> io::Result<()> {
        match v {
            Some(x) => {
                self.u8(1)?;
                self.u8(x)
            }
            None => self.u8(0),
        }
    }
    fn spec(&mut self, s: ModificationSpecificity) -> io::Result<()> {
        match s {
            ModificationSpecificity::PeptideN(r) => {
                self.u8(0)?;
                self.opt_u8(r)
            }
            ModificationSpecificity::PeptideC(r) => {
                self.u8(1)?;
                self.opt_u8(r)
            }
            ModificationSpecificity::ProteinN(r) => {
                self.u8(2)?;
                self.opt_u8(r)
            }
            ModificationSpecificity::ProteinC(r) => {
                self.u8(3)?;
                self.opt_u8(r)
            }
            ModificationSpecificity::Residue(r) => {
                self.u8(4)?;
                self.u8(r)
            }
        }
    }
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.hasher.write(&buf[..n]);
        self.len += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// A reader over the payload that checksums what it reads and refuses to read
/// past the payload length named in the header. Array counts are checked
/// against the bytes left before anything is allocated, so a corrupt count
/// cannot ask for a huge allocation.
struct PayloadReader<R: Read> {
    inner: R,
    hasher: XxHash3_64,
    remaining: u64,
}

impl<R: Read> PayloadReader<R> {
    fn take(&mut self, buf: &mut [u8]) -> io::Result<()> {
        if buf.len() as u64 > self.remaining {
            return Err(bad("payload length"));
        }
        self.inner.read_exact(buf)?;
        self.hasher.write(buf);
        self.remaining -= buf.len() as u64;
        Ok(())
    }
    fn u8(&mut self) -> io::Result<u8> {
        let mut b = [0u8; 1];
        self.take(&mut b)?;
        Ok(b[0])
    }
    fn bool(&mut self) -> io::Result<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(bad("flag")),
        }
    }
    fn u32(&mut self) -> io::Result<u32> {
        let mut b = [0u8; 4];
        self.take(&mut b)?;
        Ok(u32::from_le_bytes(b))
    }
    fn u64(&mut self) -> io::Result<u64> {
        let mut b = [0u8; 8];
        self.take(&mut b)?;
        Ok(u64::from_le_bytes(b))
    }
    fn f32(&mut self) -> io::Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }
    /// A count of elements that each take at least `min_size` bytes.
    fn count(&mut self, wide: bool, min_size: u64) -> io::Result<usize> {
        let n = if wide {
            self.u64()?
        } else {
            u64::from(self.u32()?)
        };
        if n.saturating_mul(min_size.max(1)) > self.remaining {
            return Err(bad("array length"));
        }
        usize::try_from(n).map_err(|_| bad("array length"))
    }
    fn bytes(&mut self) -> io::Result<Vec<u8>> {
        let n = self.count(false, 1)?;
        let mut v = vec![0u8; n];
        self.take(&mut v)?;
        Ok(v)
    }
    fn opt_f32(&mut self) -> io::Result<Option<f32>> {
        Ok(if self.bool()? {
            Some(self.f32()?)
        } else {
            None
        })
    }
    fn opt_u8(&mut self) -> io::Result<Option<u8>> {
        Ok(if self.bool()? { Some(self.u8()?) } else { None })
    }
    fn spec(&mut self) -> io::Result<ModificationSpecificity> {
        Ok(match self.u8()? {
            0 => ModificationSpecificity::PeptideN(self.opt_u8()?),
            1 => ModificationSpecificity::PeptideC(self.opt_u8()?),
            2 => ModificationSpecificity::ProteinN(self.opt_u8()?),
            3 => ModificationSpecificity::ProteinC(self.opt_u8()?),
            4 => ModificationSpecificity::Residue(self.u8()?),
            _ => return Err(bad("modification specificity")),
        })
    }
}

fn write_index_file(path: &Path, key: &[u8; 16], db: &IndexedDatabase) -> io::Result<u64> {
    let mut file = File::create(path)?;
    file.write_all(&[0u8; HEADER_LEN as usize])?;
    let mut w = HashingWriter {
        inner: BufWriter::with_capacity(8 << 20, &mut file),
        hasher: XxHash3_64::new(),
        len: 0,
    };
    write_payload(&mut w, db)?;
    w.flush()?;
    let (payload_len, checksum) = (w.len, w.hasher.finish());
    drop(w);

    let mut header = Vec::with_capacity(HEADER_LEN as usize);
    header.extend_from_slice(MAGIC);
    header.push(STRATEGY_FULL_INDEX);
    header.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    header.extend_from_slice(key);
    header.extend_from_slice(&payload_len.to_le_bytes());
    header.extend_from_slice(&checksum.to_le_bytes());
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&header)?;
    file.sync_all()?;
    Ok(HEADER_LEN + payload_len)
}

fn write_payload<W: Write>(w: &mut HashingWriter<W>, db: &IndexedDatabase) -> io::Result<()> {
    w.u64(db.bucket_size as u64)?;
    w.u8(db.generate_decoys as u8)?;
    w.bytes(db.decoy_tag.as_bytes())?;

    w.len32(db.ion_kinds.len())?;
    for k in &db.ion_kinds {
        w.u8(kind_code(*k))?;
    }

    w.len32(db.potential_mods.len())?;
    for (spec, mass) in &db.potential_mods {
        w.spec(*spec)?;
        w.f32(*mass)?;
    }

    w.u64(db.min_value.len() as u64)?;
    for block in db.min_value.chunks(BLOCK) {
        let mut buf = Vec::with_capacity(block.len() * 4);
        for v in block {
            buf.extend_from_slice(&v.to_bits().to_le_bytes());
        }
        w.write_all(&buf)?;
    }

    // Shared strings, stored once each. Keyed by content: two equal strings
    // come back as one shared `Arc`, which is how Sage builds them anyway.
    let mut accession_ix: HashMap<&str, u32> = HashMap::new();
    let mut accessions: Vec<&str> = Vec::new();
    let mut sequence_ix: HashMap<&[u8], u32> = HashMap::new();
    let mut sequences: Vec<&[u8]> = Vec::new();
    for p in &db.peptides {
        if !sequence_ix.contains_key(&*p.sequence) {
            sequence_ix.insert(&p.sequence, sequences.len() as u32);
            sequences.push(&p.sequence);
        }
        for a in &p.proteins {
            if !accession_ix.contains_key(&**a) {
                accession_ix.insert(a, accessions.len() as u32);
                accessions.push(a);
            }
        }
    }
    w.u64(accessions.len() as u64)?;
    for a in &accessions {
        w.bytes(a.as_bytes())?;
    }
    w.u64(sequences.len() as u64)?;
    for s in &sequences {
        w.bytes(s)?;
    }

    w.u64(db.peptides.len() as u64)?;
    for p in &db.peptides {
        w.u8(p.decoy as u8)?;
        w.u32(sequence_ix[&*p.sequence])?;
        w.len32(p.modifications.len())?;
        for m in &p.modifications {
            w.f32(*m)?;
        }
        w.opt_f32(p.nterm)?;
        w.opt_f32(p.cterm)?;
        w.f32(p.monoisotopic)?;
        w.u8(p.missed_cleavages)?;
        w.u8(p.semi_enzymatic as u8)?;
        w.u8(position_code(p.position))?;
        w.len32(p.proteins.len())?;
        for a in &p.proteins {
            w.u32(accession_ix[&**a])?;
        }
    }

    w.u64(db.fragments.len() as u64)?;
    for block in db.fragments.chunks(BLOCK) {
        let mut buf = Vec::with_capacity(block.len() * 8);
        for f in block {
            buf.extend_from_slice(&f.peptide_index.0.to_le_bytes());
            buf.extend_from_slice(&f.fragment_mz.to_bits().to_le_bytes());
        }
        w.write_all(&buf)?;
    }
    Ok(())
}

fn read_index_file(path: &Path, key: &[u8; 16]) -> io::Result<IndexedDatabase> {
    let file = File::open(path)?;
    let file_len = file.metadata()?.len();
    let mut r = BufReader::with_capacity(8 << 20, file);

    let mut header = [0u8; HEADER_LEN as usize];
    r.read_exact(&mut header)?;
    if &header[0..5] != MAGIC {
        return Err(bad("magic"));
    }
    if header[5] != STRATEGY_FULL_INDEX {
        return Err(bad("strategy"));
    }
    if u32::from_le_bytes(header[6..10].try_into().unwrap()) != FORMAT_VERSION {
        return Err(bad("format version"));
    }
    if &header[10..26] != key {
        return Err(bad("key"));
    }
    let payload_len = u64::from_le_bytes(header[26..34].try_into().unwrap());
    let checksum = u64::from_le_bytes(header[34..42].try_into().unwrap());
    if HEADER_LEN.checked_add(payload_len) != Some(file_len) {
        return Err(bad("file length"));
    }

    let mut p = PayloadReader {
        inner: r,
        hasher: XxHash3_64::new(),
        remaining: payload_len,
    };
    let db = read_payload(&mut p)?;
    if p.remaining != 0 {
        return Err(bad("trailing payload"));
    }
    if p.hasher.finish() != checksum {
        return Err(bad("checksum"));
    }
    Ok(db)
}

fn read_payload<R: Read>(r: &mut PayloadReader<R>) -> io::Result<IndexedDatabase> {
    let bucket_size = usize::try_from(r.u64()?).map_err(|_| bad("bucket size"))?;
    let generate_decoys = r.bool()?;
    let decoy_tag = String::from_utf8(r.bytes()?).map_err(|_| bad("decoy tag"))?;

    let n = r.count(false, 1)?;
    let ion_kinds = (0..n)
        .map(|_| kind_from(r.u8()?))
        .collect::<io::Result<Vec<_>>>()?;

    let n = r.count(false, 6)?;
    let potential_mods = (0..n)
        .map(|_| Ok((r.spec()?, r.f32()?)))
        .collect::<io::Result<Vec<_>>>()?;

    let n = r.count(true, 4)?;
    let mut min_value = Vec::with_capacity(n);
    let mut buf = Vec::new();
    while min_value.len() < n {
        let take = (n - min_value.len()).min(BLOCK);
        buf.resize(take * 4, 0);
        r.take(&mut buf)?;
        min_value.extend(
            buf.as_chunks::<4>()
                .0
                .iter()
                .map(|c| f32::from_bits(u32::from_le_bytes(*c))),
        );
    }

    let n = r.count(true, 4)?;
    let accessions = (0..n)
        .map(|_| {
            let s = String::from_utf8(r.bytes()?).map_err(|_| bad("accession"))?;
            Ok(Arc::<str>::from(s))
        })
        .collect::<io::Result<Vec<_>>>()?;
    let n = r.count(true, 4)?;
    let sequences = (0..n)
        .map(|_| Ok(Arc::<[u8]>::from(r.bytes()?)))
        .collect::<io::Result<Vec<_>>>()?;

    let n = r.count(true, 20)?;
    let mut peptides = Vec::with_capacity(n);
    for _ in 0..n {
        let decoy = r.bool()?;
        let seq = r.u32()? as usize;
        let sequence = sequences
            .get(seq)
            .ok_or_else(|| bad("sequence index"))?
            .clone();
        let m = r.count(false, 4)?;
        let modifications = (0..m).map(|_| r.f32()).collect::<io::Result<Vec<_>>>()?;
        let nterm = r.opt_f32()?;
        let cterm = r.opt_f32()?;
        let monoisotopic = r.f32()?;
        let missed_cleavages = r.u8()?;
        let semi_enzymatic = r.bool()?;
        let position = position_from(r.u8()?)?;
        let k = r.count(false, 4)?;
        let proteins = (0..k)
            .map(|_| {
                let i = r.u32()? as usize;
                accessions
                    .get(i)
                    .cloned()
                    .ok_or_else(|| bad("accession index"))
            })
            .collect::<io::Result<Vec<_>>>()?;
        peptides.push(Peptide {
            decoy,
            sequence,
            modifications,
            nterm,
            cterm,
            monoisotopic,
            missed_cleavages,
            semi_enzymatic,
            position,
            proteins,
        });
    }

    let n = r.count(true, 8)?;
    let peptide_count = peptides.len();
    let mut fragments = Vec::with_capacity(n);
    while fragments.len() < n {
        let take = (n - fragments.len()).min(BLOCK);
        buf.resize(take * 8, 0);
        r.take(&mut buf)?;
        for c in buf.as_chunks::<8>().0 {
            let ix = u32::from_le_bytes(c[0..4].try_into().unwrap());
            if ix as usize >= peptide_count {
                return Err(bad("fragment peptide index"));
            }
            fragments.push(Theoretical {
                peptide_index: PeptideIx(ix),
                fragment_mz: f32::from_bits(u32::from_le_bytes(c[4..8].try_into().unwrap())),
            });
        }
    }

    Ok(IndexedDatabase {
        peptides,
        fragments,
        ion_kinds,
        min_value,
        potential_mods,
        bucket_size,
        generate_decoys,
        decoy_tag,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sage_core::database::Builder;
    use sage_core::fasta::Fasta;

    /// Three small proteins. P1 and P2 share the tryptic peptide `SAMPLER`,
    /// and P2 and P3 share `LMNPQR`, so some peptides carry two accessions.
    /// The variable Met oxidation gives one sequence two modforms.
    const FASTA: &str = ">sp|P1|ONE\nMKSAMPLERGATTACAKPEPTIDEMRSEQ\n\
                         >sp|P2|TWO\nMKSAMPLERKLMNPQRSTVWYKACDEFGHIK\n\
                         >sp|P3|THREE\nMADEKGHIKLMNPQRSTAWYKEEEEEEEK\n";

    fn builder(extra: serde_json::Value) -> Builder {
        let mut v = serde_json::json!({
            "bucket_size": 128,
            "enzyme": { "missed_cleavages": 1, "min_len": 4, "max_len": 30,
                        "cleave_at": "KR", "restrict": "P" },
            "peptide_min_mass": 300.0,
            "peptide_max_mass": 5000.0,
            "static_mods": { "C": 57.0215 },
            "variable_mods": { "M": [15.9949] },
            "max_variable_mods": 2,
            "fasta": "/tmp/one.fasta"
        });
        if let (Some(obj), Some(more)) = (v.as_object_mut(), extra.as_object()) {
            for (k, val) in more {
                obj.insert(k.clone(), val.clone());
            }
        }
        serde_json::from_value(v).expect("test Builder JSON")
    }

    fn params(extra: serde_json::Value) -> Parameters {
        builder(extra).make_parameters()
    }

    fn build_db(p: &Parameters) -> IndexedDatabase {
        let fasta = Fasta::parse(FASTA.to_string(), p.decoy_tag.clone(), p.generate_decoys);
        p.clone().build(fasta)
    }

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "sagegui-index-cache-test-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn cache_in(dir: &Path, key_byte: u8) -> IndexCache {
        let key = [key_byte; 16];
        IndexCache {
            dir: dir.to_path_buf(),
            path: dir.join(format!(
                "{FILE_PREFIX}{:032x}{FILE_SUFFIX}",
                u128::from_le_bytes(key)
            )),
            key,
        }
    }

    fn peptide_bits(p: &Peptide) -> Vec<u32> {
        let mut v = vec![p.monoisotopic.to_bits()];
        v.extend(p.modifications.iter().map(|m| m.to_bits()));
        v.extend(p.nterm.map(f32::to_bits));
        v.extend(p.cterm.map(f32::to_bits));
        v
    }

    /// The guard that fails loudly when Sage adds a field: the struct
    /// literals in `read_payload` stop compiling, and this test compares
    /// every field, floats by their bits.
    #[test]
    fn round_trip_preserves_every_field() {
        let p = params(serde_json::json!({}));
        let db = build_db(&p);
        assert!(db.peptides.len() > 10, "fixture too small to mean anything");
        assert!(db.peptides.iter().any(|p| p.proteins.len() > 1));
        assert!(db
            .peptides
            .iter()
            .any(|p| p.modifications.iter().any(|m| *m != 0.0)));
        assert!(db.peptides.iter().any(|p| p.decoy));

        let dir = temp_dir("roundtrip");
        let cache = cache_in(&dir, 7);
        assert!(matches!(cache.store(&db), StoreOutcome::Stored { .. }));
        let back = cache.load().expect("the stored file must load");

        assert!(back.peptides == db.peptides);
        for (a, b) in back.peptides.iter().zip(&db.peptides) {
            assert_eq!(peptide_bits(a), peptide_bits(b));
        }
        assert_eq!(back.fragments.len(), db.fragments.len());
        for (a, b) in back.fragments.iter().zip(&db.fragments) {
            assert_eq!(a.peptide_index, b.peptide_index);
            assert_eq!(a.fragment_mz.to_bits(), b.fragment_mz.to_bits());
        }
        assert_eq!(back.ion_kinds, db.ion_kinds);
        let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        assert_eq!(bits(&back.min_value), bits(&db.min_value));
        assert_eq!(back.potential_mods.len(), db.potential_mods.len());
        for (a, b) in back.potential_mods.iter().zip(&db.potential_mods) {
            assert_eq!(a.0, b.0);
            assert_eq!(a.1.to_bits(), b.1.to_bits());
        }
        assert_eq!(back.bucket_size, db.bucket_size);
        assert_eq!(back.generate_decoys, db.generate_decoys);
        assert_eq!(back.decoy_tag, db.decoy_tag);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn shared_accessions_and_sequences_are_shared_after_load() {
        let db = build_db(&params(serde_json::json!({})));
        let dir = temp_dir("arc");
        let cache = cache_in(&dir, 8);
        cache.store(&db);
        let back = cache.load().unwrap();

        let mut by_accession: HashMap<&str, Vec<&Arc<str>>> = HashMap::new();
        let mut by_sequence: HashMap<&[u8], Vec<&Arc<[u8]>>> = HashMap::new();
        for p in &back.peptides {
            by_sequence
                .entry(&p.sequence)
                .or_default()
                .push(&p.sequence);
            for a in &p.proteins {
                by_accession.entry(a).or_default().push(a);
            }
        }
        let shared_acc = by_accession.values().filter(|v| v.len() > 1).count();
        let shared_seq = by_sequence.values().filter(|v| v.len() > 1).count();
        assert!(shared_acc > 0 && shared_seq > 0, "fixture must repeat both");
        for group in by_accession.values() {
            assert!(group.iter().all(|a| Arc::ptr_eq(a, group[0])));
        }
        for group in by_sequence.values() {
            assert!(group.iter().all(|s| Arc::ptr_eq(s, group[0])));
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_key_does_not_depend_on_map_order() {
        let a = params(serde_json::json!({
            "static_mods": { "C": 57.0215, "K": 8.0142, "R": 10.0083 },
            "variable_mods": { "M": [15.9949], "^Q": [-17.0265], "S": [79.9663] }
        }));
        let b = params(serde_json::json!({
            "static_mods": { "R": 10.0083, "C": 57.0215, "K": 8.0142 },
            "variable_mods": { "S": [79.9663], "M": [15.9949], "^Q": [-17.0265] }
        }));
        for _ in 0..5 {
            assert_eq!(
                cache_key(&a, 1).unwrap(),
                cache_key(&b, 1).unwrap(),
                "HashMap order must not reach the key"
            );
        }
    }

    #[test]
    fn cache_key_changes_when_a_database_parameter_changes() {
        let base = cache_key(&params(serde_json::json!({})), 1).unwrap();
        let changes = [
            serde_json::json!({ "enzyme": { "missed_cleavages": 2, "min_len": 4,
                "max_len": 30, "cleave_at": "KR", "restrict": "P" } }),
            serde_json::json!({ "enzyme": { "missed_cleavages": 1, "min_len": 4,
                "max_len": 30, "cleave_at": "KR", "restrict": "P", "semi_enzymatic": true } }),
            serde_json::json!({ "max_variable_mods": 3 }),
            serde_json::json!({ "static_mods": { "C": 57.02 } }),
            serde_json::json!({ "variable_mods": { "M": [15.9949], "C": [1.0] } }),
            serde_json::json!({ "generate_decoys": false }),
            serde_json::json!({ "bucket_size": 256 }),
            serde_json::json!({ "decoy_tag": "decoy_" }),
            serde_json::json!({ "peptide_min_mass": 400.0 }),
            serde_json::json!({ "peptide_max_mass": 4000.0 }),
            serde_json::json!({ "min_ion_index": 3 }),
            serde_json::json!({ "ion_kinds": ["b", "y", "a"] }),
        ];
        for change in changes {
            let key = cache_key(&params(change.clone()), 1).unwrap();
            assert_ne!(key, base, "changing {change} must change the key");
        }
        assert_ne!(
            cache_key(&params(serde_json::json!({})), 2).unwrap(),
            base,
            "different FASTA content must change the key"
        );
    }

    /// The main reason the cache exists: a re-run after changing only a
    /// tolerance or a file selection must hit. Those settings live outside
    /// `Parameters`, so they cannot reach the key. Inside it, only the FASTA
    /// path and the prefilter settings must be ignored.
    #[test]
    fn cache_key_ignores_what_does_not_change_the_database() {
        let base = cache_key(&params(serde_json::json!({})), 1).unwrap();
        let same = [
            serde_json::json!({ "fasta": "/somewhere/else/renamed.fasta" }),
            serde_json::json!({ "prefilter_chunk_size": 5000 }),
            serde_json::json!({ "prefilter_low_memory": false }),
        ];
        for change in same {
            let key = cache_key(&params(change.clone()), 1).unwrap();
            assert_eq!(key, base, "{change} must not change the key");
        }
    }

    #[test]
    fn prefiltering_or_a_cloud_fasta_turns_caching_off() {
        let dir = temp_dir("off");
        let fasta = dir.join("one.fasta");
        fs::write(&fasta, FASTA).unwrap();
        let path = fasta.to_string_lossy().to_string();

        let on = params(serde_json::json!({ "fasta": path }));
        let prefilter = params(serde_json::json!({ "fasta": path, "prefilter": true }));
        let cloud = params(serde_json::json!({ "fasta": "s3://bucket/one.fasta" }));
        assert!(IndexCache::in_dir(&on, dir.clone()).is_ok());
        assert!(IndexCache::for_parameters(&on, false).is_none());
        assert!(IndexCache::for_parameters(&prefilter, true).is_none());
        assert!(IndexCache::for_parameters(&cloud, true).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_bad_cache_file_is_a_miss_and_never_panics() {
        let db = build_db(&params(serde_json::json!({})));
        let dir = temp_dir("corrupt");
        let cache = cache_in(&dir, 9);
        cache.store(&db);
        let good = fs::read(&cache.path).unwrap();

        let mut cases: Vec<(&str, Vec<u8>)> = vec![
            ("empty", Vec::new()),
            ("header only", good[..HEADER_LEN as usize].to_vec()),
            ("truncated", good[..good.len() - 3].to_vec()),
            ("one extra byte", [good.clone(), vec![0]].concat()),
        ];
        let mut flip = |name, at: usize| {
            let mut v = good.clone();
            v[at] ^= 0x40;
            cases.push((name, v));
        };
        flip("wrong magic", 0);
        flip("wrong strategy", 5);
        flip("wrong format version", 6);
        flip("wrong key", 12);
        flip("wrong checksum", 36);
        flip("payload bit early", HEADER_LEN as usize + 2);
        flip("payload bit late", good.len() - 2);
        flip("payload bit middle", good.len() / 2);

        for (name, bytes) in cases {
            fs::write(&cache.path, &bytes).unwrap();
            assert!(cache.load().is_none(), "{name} must be a miss");
        }
        fs::remove_file(&cache.path).unwrap();
        assert!(cache.load().is_none(), "a missing file must be a miss");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_huge_count_in_a_corrupt_file_does_not_allocate() {
        // A payload that claims 2^60 fragments but carries a few bytes. The
        // count check must refuse it before `Vec::with_capacity`.
        let mut payload = Vec::new();
        payload.extend_from_slice(&(1u64 << 60).to_le_bytes());
        payload.extend_from_slice(&[0u8; 16]);
        let mut r = PayloadReader {
            inner: &payload[..],
            hasher: XxHash3_64::new(),
            remaining: payload.len() as u64,
        };
        assert!(r.count(true, 8).is_err());
    }

    #[test]
    fn an_entry_over_the_size_limit_is_not_written() {
        let db = build_db(&params(serde_json::json!({})));
        let dir = temp_dir("toolarge");
        let cache = cache_in(&dir, 10);
        let outcome = cache.store_with_limits(&db, 100, MAX_CACHE_BYTES);
        assert!(matches!(outcome, StoreOutcome::TooLarge { .. }));
        assert_eq!(stats_in(&dir).files, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn eviction_removes_the_least_recently_used_entry() {
        let db = build_db(&params(serde_json::json!({})));
        let dir = temp_dir("lru");
        let (a, b, c) = (cache_in(&dir, 1), cache_in(&dir, 2), cache_in(&dir, 3));
        let StoreOutcome::Stored { bytes } = a.store(&db) else {
            panic!("store failed")
        };
        b.store(&db);
        // Make `a` the oldest, then use `b` so it is the newest.
        let old = SystemTime::now() - std::time::Duration::from_secs(3600);
        File::options()
            .write(true)
            .open(&a.path)
            .unwrap()
            .set_modified(old)
            .unwrap();
        assert!(b.load().is_some());

        // Room for two entries in total: storing `c` must drop `a` only.
        let outcome = c.store_with_limits(&db, MAX_ENTRY_BYTES, 2 * bytes + bytes / 2);
        assert!(matches!(outcome, StoreOutcome::Stored { .. }));
        assert!(!a.path.exists(), "the least recently used entry goes first");
        assert!(b.path.exists() && c.path.exists());
        assert_eq!(stats_in(&dir).files, 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn clear_deletes_only_cache_files() {
        let db = build_db(&params(serde_json::json!({})));
        let dir = temp_dir("clear");
        cache_in(&dir, 4).store(&db);
        fs::write(dir.join(".tmp-1-2"), b"partial").unwrap();
        fs::write(dir.join("keep-me.txt"), b"not ours").unwrap();
        let before = stats_in(&dir);
        assert_eq!(before.files, 1);

        let freed = clear_in(&dir).unwrap();
        assert_eq!(freed, before.bytes + "partial".len() as u64);
        assert_eq!(stats_in(&dir), CacheStats::default());
        assert!(dir.join("keep-me.txt").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn estimate_is_an_upper_bound() {
        let db = build_db(&params(serde_json::json!({})));
        let dir = temp_dir("estimate");
        let StoreOutcome::Stored { bytes } = cache_in(&dir, 5).store(&db) else {
            panic!("store failed")
        };
        assert!(estimate_size(&db) >= bytes);
        let _ = fs::remove_dir_all(&dir);
    }

    /// End to end on real data: a search on a database loaded from the cache
    /// must give the same PSMs, scores and quant as a search on a fresh build.
    /// Needs a Sage `results.json` (or `config.json`) whose FASTA and spectra
    /// are on this machine, so it is ignored by default. Run it with:
    ///
    /// `SAGEGUI_CACHE_E2E=/path/to/results.json cargo test --release \
    ///  cached_database_gives_the_same_search_results -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn cached_database_gives_the_same_search_results() {
        use sage_cli::input::Input;
        use sage_cli::runner::Runner;
        use std::time::Instant;

        let Ok(config) = std::env::var("SAGEGUI_CACHE_E2E") else {
            eprintln!("SAGEGUI_CACHE_E2E is not set; nothing to do");
            return;
        };
        let root = temp_dir("e2e");
        let run = |name: &str| {
            let mut input = Input::load(&config).expect("config must load");
            let out = root.join(name);
            input.output_directory = Some(out.to_string_lossy().to_string());
            (input.build().expect("config must build"), out)
        };
        let parallel = (num_cpus::get() / 2).max(1);

        let (search, fresh_out) = run("fresh");
        let cache = IndexCache::in_dir(&search.database, root.join("cache")).unwrap();
        let t = Instant::now();
        let runner = Runner::new(search, parallel).unwrap();
        let build_time = t.elapsed();
        let t = Instant::now();
        let StoreOutcome::Stored { bytes } = cache.store(&runner.database) else {
            panic!("store failed")
        };
        let store_time = t.elapsed();
        runner.run(parallel, false).unwrap();

        let (search, cached_out) = run("cached");
        let again = IndexCache::in_dir(&search.database, root.join("cache")).unwrap();
        assert_eq!(
            again.path, cache.path,
            "same settings must give the same key"
        );
        let t = Instant::now();
        let db = again.load().expect("the entry just written must load");
        let load_time = t.elapsed();
        Runner::from_parts(search, db).run(parallel, false).unwrap();

        eprintln!(
            "build {build_time:.1?}, store {store_time:.1?} ({}), load {load_time:.1?}",
            size_text(bytes)
        );

        let rows = |path: PathBuf| -> HashMap<String, String> {
            let text = fs::read_to_string(&path).unwrap();
            let mut lines = text.lines();
            let header: Vec<&str> = lines.next().unwrap().split('\t').collect();
            let id = header.iter().position(|h| *h == "psm_id");
            let col = |name: &str| header.iter().position(|h| *h == name);
            let (f, s, p, z) = (
                col("filename"),
                col("scannr"),
                col("peptide"),
                col("charge"),
            );
            lines
                .map(|line| {
                    let cells: Vec<&str> = line.split('\t').collect();
                    let pick = |c: Option<usize>| c.map(|i| cells[i]).unwrap_or("");
                    let key = [pick(f), pick(s), pick(p), pick(z)].join("|");
                    let rest: Vec<&str> = cells
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| Some(*i) != id)
                        .map(|(_, c)| *c)
                        .collect();
                    (key, rest.join("\t"))
                })
                .collect()
        };
        let fresh = rows(fresh_out.join("results.sage.tsv"));
        let cached = rows(cached_out.join("results.sage.tsv"));
        assert!(!fresh.is_empty());
        assert_eq!(fresh.len(), cached.len(), "PSM count");
        let differing = fresh
            .iter()
            .filter(|(k, v)| cached.get(*k) != Some(v))
            .count();
        assert_eq!(differing, 0, "PSMs that differ in any column except psm_id");
        eprintln!(
            "{} PSMs identical in every column except psm_id",
            fresh.len()
        );

        let lfq = |dir: &Path| {
            let mut lines: Vec<String> = fs::read_to_string(dir.join("lfq.tsv"))
                .map(|t| t.lines().map(str::to_string).collect())
                .unwrap_or_default();
            lines.sort();
            lines
        };
        assert_eq!(lfq(&fresh_out), lfq(&cached_out), "lfq.tsv, sorted");
        let _ = fs::remove_dir_all(&root);
    }
}
