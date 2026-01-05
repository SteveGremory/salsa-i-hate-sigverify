use solana_clock::Slot;
use solana_perf::packet::PacketBatch;

/// A harmonic block contains transactions intended for a specific slot.
#[derive(Clone, Debug)]
pub struct HarmonicBlock {
    transactions: PacketBatch,
    intended_slot: Slot,
}

impl HarmonicBlock {
    pub fn new(transactions: PacketBatch, intended_slot: Slot) -> Self {
        Self {
            transactions,
            intended_slot,
        }
    }

    pub fn transactions(&self) -> &PacketBatch {
        &self.transactions
    }

    pub fn intended_slot(&self) -> Slot {
        self.intended_slot
    }

    pub fn take(self) -> PacketBatch {
        self.transactions
    }
}
