use std::collections::BTreeMap;
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineConfig {
    pub bgzf_threads: usize,
    pub filter_threads: usize,
    pub batch_records: usize,
    pub queue_batches: usize,
}

impl PipelineConfig {
    pub fn enabled(&self) -> bool {
        self.bgzf_threads > 1 || self.filter_threads > 1
    }

    pub fn bounded_capacity(&self) -> usize {
        self.batch_records.saturating_mul(self.queue_batches.max(1))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordBatch {
    pub sequence: u64,
    pub bytes: Vec<u8>,
    pub record_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedBatch {
    pub sequence: u64,
    pub bytes: Vec<u8>,
}

pub fn write_ordered_batches<W, I>(writer: &mut W, batches: I) -> io::Result<()>
where
    W: Write + ?Sized,
    I: IntoIterator<Item = AcceptedBatch>,
{
    let mut next_sequence = 0_u64;
    let mut pending = BTreeMap::<u64, Vec<u8>>::new();

    for batch in batches {
        if batch.sequence < next_sequence || pending.insert(batch.sequence, batch.bytes).is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "duplicate accepted batch sequence",
            ));
        }

        while let Some(bytes) = pending.remove(&next_sequence) {
            writer.write_all(&bytes)?;
            next_sequence += 1;
        }
    }

    if pending.is_empty() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "accepted batch sequence gap",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_writer_flushes_batches_in_sequence_order() {
        let batches = [
            AcceptedBatch {
                sequence: 2,
                bytes: b"third\n".to_vec(),
            },
            AcceptedBatch {
                sequence: 0,
                bytes: b"first\n".to_vec(),
            },
            AcceptedBatch {
                sequence: 1,
                bytes: b"second\n".to_vec(),
            },
        ];
        let mut output = Vec::new();

        write_ordered_batches(&mut output, batches).unwrap();

        assert_eq!(output, b"first\nsecond\nthird\n");
    }
}
