use std::{
    collections::{HashMap, HashSet},
    fs, usize,
};
use types::{Epoch, EthSpec, MainnetEthSpec, SignedBlindedBeaconBlock, Slot};
use unicode_segmentation::UnicodeSegmentation;

type T = MainnetEthSpec;

const FROM_BLOCK: usize = 6064198;
const TO_BLOCK: usize = 7165798;
const BLOCKS_PATH: &str = "/home/lion/code/dapplion/beacon_block_downloader/blocks";

fn main() {
    let spec = T::default_spec();

    // No need to track actual indexes, track (slot, committee_index, bit_index)
    let mut previous_participation = HashSet::new();
    let mut current_participation = HashSet::new();
    let mut participation_epoch = Epoch::new(0);

    let mut not_lodestar_ok = 0f32;
    let mut not_lodestar_bad = 0f32;
    let mut yes_lodestar_ok = 0f32;
    let mut yes_lodestar_bad = 0f32;

    // Lodestar block packing can include a max of attestations with same data root
    const MAX_ATTESTATIONS_PER_GROUP: usize = 2;
    const SLOTS_PER_EPOCH_SQRT: Slot = Slot::new(5);

    for slot in FROM_BLOCK..TO_BLOCK {
        let block = match fs::read(format!("{}/block_mainnet_{}.ssz", BLOCKS_PATH, slot)) {
            Ok(block) => block,
            Err(_) => continue,
        };
        let block = SignedBlindedBeaconBlock::<T>::from_ssz_bytes(&block, &spec).unwrap();

        let block_slot = block.message().slot();
        let graffiti = block.message().body().graffiti().as_utf8_lossy();
        let attestations = block.message().body().attestations();

        // Roll participation
        let block_epoch = block_slot.epoch(T::slots_per_epoch());
        if block_epoch > participation_epoch + 1 {
            previous_participation.clear();
            current_participation.clear();
        } else if block_epoch == participation_epoch + 1 {
            previous_participation = current_participation.clone();
            current_participation.clear();
        }
        participation_epoch = block_epoch;

        // Count data root instances
        let mut data_root_count = HashMap::new();
        let mut scores = vec![];

        // Account for participation with only the view of the pre-state
        for attestation in attestations {
            *data_root_count.entry(&attestation.data).or_insert(0) += 1;

            let participation = if attestation.data.slot.epoch(T::slots_per_epoch()) == block_epoch
            {
                &mut current_participation
            } else {
                &mut previous_participation
            };

            let mut not_seen_participants = 0;

            for (i, participant) in attestation.aggregation_bits.iter().enumerate() {
                if participant
                    && !participation.contains(&(attestation.data.slot, attestation.data.index, i))
                {
                    // Do not mutate participation, do after accounting for all attestations in
                    // block
                    not_seen_participants += 1
                }
            }

            scores.push(
                not_seen_participants as f64 / (block_slot - attestation.data.slot).as_u64() as f64,
            );
        }

        // After counting attestation scores, update state participation
        for attestation in attestations {
            // Only attestations with TIMELY_SOURCE are considered participant
            // if (isMatchingSource && inclusionDelay <= SLOTS_PER_EPOCH_SQRT) flags |= TIMELY_SOURCE;
            let is_timely_source = (block_slot - attestation.data.slot) <= SLOTS_PER_EPOCH_SQRT;
            if is_timely_source {
                let participation =
                    if attestation.data.slot.epoch(T::slots_per_epoch()) == block_epoch {
                        &mut current_participation
                    } else {
                        &mut previous_participation
                    };

                for (i, participant) in attestation.aggregation_bits.iter().enumerate() {
                    if participant
                        && !participation.contains(&(
                            attestation.data.slot,
                            attestation.data.index,
                            i,
                        ))
                    {
                        participation.insert((attestation.data.slot, attestation.data.index, i));
                    }
                }
            }
        }

        let has_more_than_two = data_root_count
            .values()
            .any(|&count| count > MAX_ATTESTATIONS_PER_GROUP);

        let scores_sorted = is_sorted_desc(&scores);

        let is_lodestar_graffiti = graffiti.to_lowercase().contains("lodestar");
        let is_lodestar_algo = !has_more_than_two && scores_sorted;

        match (is_lodestar_graffiti, is_lodestar_algo) {
            (true, true) => yes_lodestar_ok += 1.,
            (true, false) => yes_lodestar_bad += 1.,
            (false, true) => not_lodestar_bad += 1.,
            (false, false) => not_lodestar_ok += 1.,
        };

        if true {
            println!(
                "{:<8} {:<8} {:<8} {:<8}",
                yes_lodestar_ok / (yes_lodestar_ok + yes_lodestar_bad),
                yes_lodestar_bad / (yes_lodestar_ok + yes_lodestar_bad),
                not_lodestar_ok / (not_lodestar_ok + not_lodestar_bad),
                not_lodestar_bad / (not_lodestar_ok + not_lodestar_bad),
            );
        }

        if false {
            println!(
                "{:<8} graffiti '{:<32}' too_many {} sorted {}",
                block_slot,
                remove_emoji(&graffiti),
                has_more_than_two,
                scores_sorted,
            );
            if graffiti.to_lowercase().contains("lodestar") {
                println!("{:?}", scores)
            }
        }

        if is_lodestar_graffiti && !scores_sorted {
            println!("{:?}", scores)
        }
    }
}

fn is_sorted_desc(arr: &[f64]) -> bool {
    arr.windows(2).all(|w| w[0] >= w[1])
}

pub fn remove_emoji(string: &str) -> String {
    let graphemes = string.graphemes(true);

    let is_not_emoji = |x: &&str| emojis::get(x).is_none();

    graphemes.filter(is_not_emoji).collect()
}
