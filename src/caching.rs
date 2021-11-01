use crate::global::*;
use crate::rules;
use crate::strategy::{self, persistent_caches};
use crate::yahtzee_bonus_rules as bonus;

use std::fs::{read, write};
use std::io::{Read, Write};
use std::iter::repeat;

use anyhow::{anyhow, Result};
use flate2::{read::DeflateDecoder, write::DeflateEncoder, Compression};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::{from_slice, to_vec};

/// Persistent caches
#[derive(Serialize, Deserialize)]
struct Caches {
    /// Version information for compatibility
    version: String,
    /// Actual caches
    caches: persistent_caches::Caches,
}

/// Populate all caches by (transitively) calling all cachable functions with their entire domains
fn warm_up_caches() {
    repeat(false)
        .zip(bonus::ALL_VARIANTS.iter().cloned())
        .chain([(true, bonus::NONE)].iter().cloned())
        .for_each(|(extreme, yahtzee_bonus)| {
            let rules = rules::build_rules(extreme, yahtzee_bonus);
            let state = strategy::State {
                score: [0, 0],
                used: [
                    vec![false; rules.fields[US].len()],
                    vec![false; rules.fields[LS].len()],
                ],
                scored_yahtzee: false,
                chips: rules.chips,
            };
            strategy::choose_reroll(state, vec![], THROWS, &rules);
        });
}

/// Dump caches to file
/// # Arguments
/// * `filename` - to dump to
/// # Returns
/// Result - serialization, I/O can fail
fn dump_caches(filename: &str) -> Result<()> {
    let caches = Caches {
        version: String::from(env!("CARGO_PKG_VERSION", "must build with cargo")),
        caches: persistent_caches::dump_caches(),
    };
    let serialized = to_vec(&caches)?;
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&serialized)?;
    let compressed = encoder.finish()?;
    write(filename, compressed)?;
    Ok(())
}

/// Populate all caches and dump to specified file
/// See `dump_caches` for signature
// TODO use
#[allow(dead_code)]
fn pre_cache(filename: &str) -> Result<()> {
    warm_up_caches();
    dump_caches(filename)
}

// TODO what happens when deserializing f64's on a f32 target?
