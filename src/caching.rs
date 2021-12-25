use crate::global::*;
use crate::rules;
use crate::strategy::{self, persistent_caches};
use crate::yahtzee_bonus_rules as bonus;

use std::fs::{write, File};
use std::io::{BufReader, Read, Write};
use std::iter::repeat;

use anyhow::{ensure, Result};
use flate2::{bufread::DeflateDecoder, write::DeflateEncoder, Compression};
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
            let state = strategy::State::new_from_rules(&rules);
            let hand = PartialHand(Vec::new());
            strategy::choose_reroll(&state, &hand, REROLLS, &rules);
        });
}

/// Dump caches to file
/// # Arguments
/// * `filename` - to dump to
/// # Returns
/// Result - serialization, I/O can fail
fn dump_caches(filename: &str) -> Result<()> {
    let caches = Caches {
        version: String::from(crate_version!()),
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
pub fn pre_cache(filename: &str) -> Result<()> {
    warm_up_caches();
    dump_caches(filename)
}

/// Restore caches from file
/// # Arguments
/// * `filename` - to restore from
/// # Returns
/// Result - decompression, deserialization, I/O can fail
pub fn restore_caches(filename: &str) -> Result<()> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let mut deflater = DeflateDecoder::new(reader);
    let mut serialized = Vec::new();
    deflater.read_to_end(&mut serialized)?;
    let caches: Caches = from_slice(&serialized)?;

    let version = crate_version!();
    let mut req = VersionReq::parse(&format!("~{}", caches.version))?;
    // Minor releases are forwards and backwards compatible
    req.comparators[0].patch = Some(0);
    ensure!(
        req.matches(&Version::parse(version)?),
        "Caches were created on version {}, this is version {}",
        caches.version,
        version
    );

    persistent_caches::populate_caches(caches.caches);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;
    use std::env::temp_dir;
    use std::fs::remove_file;

    use rayon::iter::{IntoParallelIterator, ParallelIterator};

    macro_rules! assert_subset {
        ($cache:expr, $comparison:expr) => {
            $cache
                .into_par_iter()
                .for_each(|(k, v)| assert_eq!($comparison.get(&k).unwrap(), &v));
        };
    }

    #[test]
    fn test_dump_caches() {
        // Smallest possible call to all cached strategy functions
        let rules = rules::build_rules(false, bonus::FORCED_JOKER);
        let us_fields = rules.fields[US].len();
        let mut state = strategy::State::new_from_rules(&rules);
        state.used = [
            vec![true; rules.fields[US].len()],
            [[true].repeat(us_fields - 1), vec![false]].concat(),
        ];
        let hand = PartialHand(Vec::new());
        let rerolls = 1;
        let reroll_recomm = strategy::choose_reroll(&state, &hand, rerolls, &rules);
        let reroll_key = format!(
            "{}{}{},{}",
            state.compact_fmt(),
            rules.short_name,
            hand.compact_fmt(),
            rerolls
        );

        let test_file = temp_dir().join("optimal-yahtzee-extreme-test_dump_caches");
        let test_filename = test_file.to_str().unwrap();
        dump_caches(test_filename).unwrap();

        let file = File::open(test_filename).unwrap();
        let reader = BufReader::new(file);
        let mut deflater = DeflateDecoder::new(reader);
        let mut serialized = Vec::new();
        deflater.read_to_end(&mut serialized).unwrap();
        let caches: Caches = from_slice(&serialized).unwrap();

        assert_eq!(caches.version, crate_version!());
        let comparison = persistent_caches::dump_caches();
        // Because other test functions might have modified the caches,
        // check for subset rather than equality
        assert_subset!(
            caches.caches.probability_to_roll,
            comparison.probability_to_roll
        );
        assert_subset!(
            caches.caches.choose_reroll.clone(),
            comparison.choose_reroll
        );
        assert_subset!(caches.caches.choose_field, comparison.choose_field);
        let cached_reroll = caches.caches.choose_reroll.get(&reroll_key).unwrap();
        assert_eq!(cached_reroll, &reroll_recomm);

        remove_file(test_filename).unwrap();
    }

    fn write_caches(filename: &str, caches: &Caches) {
        let serialized = to_vec(caches).unwrap();
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&serialized).unwrap();
        let compressed = encoder.finish().unwrap();
        write(filename, compressed).unwrap();
    }

    #[test]
    fn test_restore_caches() {
        // Dummy test data
        let hand = PartialHand(Vec::new());
        let rules = rules::build_rules(false, bonus::FORCED_JOKER);
        let probabilities_to_roll = strategy::ProbabilitiesToRoll {
            table: HashMap::new(),
        };
        let mut state = strategy::State::new_from_rules(&rules);
        state.used = [
            vec![true; rules.fields[US].len()],
            vec![true; rules.fields[LS].len()],
        ];
        let rerolls = 0;
        let expectation = 0.0;
        let reroll_recomm = strategy::RerollRecomm {
            hand: hand.clone(),
            state: state.clone(),
            expectation,
        };
        let field_recomm = strategy::FieldRecomm {
            section: US,
            field: 0,
            state: state.clone(),
            expectation,
        };

        // Save some `rustfmt` lines
        let roll_key = String::from(rules.short_name);
        let roll_test = probabilities_to_roll.clone();
        let reroll_key = format!(
            "{}{}{},{}",
            state.compact_fmt(),
            rules.short_name,
            hand.compact_fmt(),
            rerolls
        );
        let reroll_recomm_test = reroll_recomm.clone();
        let field_key = format!(
            "{}{}{}",
            state.compact_fmt(),
            rules.short_name,
            hand.compact_fmt(),
        );
        let field_recomm_test = field_recomm.clone();

        // Set another patch level, should still work
        let mut version = Version::parse(crate_version!()).unwrap();
        version.patch += 1;

        let mut caches = Caches {
            version: version.to_string(),
            caches: persistent_caches::Caches {
                // simple nonsense results -- have nothing, get nothing
                probability_to_roll: [(roll_key, roll_test)].iter().cloned().collect(),
                choose_reroll: [(reroll_key, reroll_recomm_test)].iter().cloned().collect(),
                choose_field: [(field_key, field_recomm_test)].iter().cloned().collect(),
            },
        };

        let test_file = temp_dir().join("optimal-yahtzee-extreme-test_restore_caches");
        let test_filename = test_file.to_str().unwrap();

        write_caches(test_filename, &caches);
        restore_caches(test_filename).unwrap();

        assert_eq!(
            strategy::probability_to_roll(hand.clone(), &rules.dice),
            probabilities_to_roll,
        );
        assert_eq!(
            strategy::choose_reroll(&state, &hand, rerolls, &rules),
            reroll_recomm,
        );
        assert_eq!(strategy::choose_field(&state, &hand, &rules), field_recomm);

        // Test version mismatch
        version.minor += 1;
        caches.version = version.to_string();
        write_caches(test_filename, &caches);
        assert!(restore_caches(test_filename).is_err());

        remove_file(test_filename).unwrap();
    }
}
