# beacon_block_lodestar_classifier

Emulates Lodestar attestation packing algorithm to guess if a block was produced by Lodestar or not

https://github.com/ChainSafe/lodestar/blob/1b40a919e3c5fc882588a1d0fd0c3adf755dc939/packages/beacon-node/src/chain/opPools/aggregatedAttestationPool.ts#L169

Summarized in rules:

- Define validator as already having participated as having the flag isTimelySouce == true
- Participation is computed from the state.participation at the attestation’s epoch
- There is at max two attestations in a block with the same data root
- Define score as `score = not_seen_attesters / (state.slot - slot)`
- Attestations are perfectly sorted in descending score order
