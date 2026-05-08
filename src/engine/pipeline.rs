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
pub struct DecodedBlock {
    pub block_sequence: u64,
    pub virtual_offset: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineCarry {
    pub pending: Vec<u8>,
    pub next_batch_sequence: u64,
}

impl LineCarry {
    pub fn push_block(&mut self, block: DecodedBlock, batch_records: usize) -> Vec<RecordBatch> {
        self.pending.extend_from_slice(&block.bytes);
        self.drain_full_batches(batch_records.max(1))
    }

    pub fn finish(&mut self) -> Vec<RecordBatch> {
        if self.pending.is_empty() {
            return Vec::new();
        };

        let bytes = std::mem::take(&mut self.pending);
        let record_count = count_records(&bytes);

        if record_count == 0 {
            Vec::new()
        } else {
            vec![self.next_batch(bytes, record_count)]
        }
    }

    fn drain_full_batches(&mut self, batch_records: usize) -> Vec<RecordBatch> {
        let mut batches = Vec::new();

        while let Some(batch_end) = nth_record_end(&self.pending, batch_records) {
            let bytes: Vec<u8> = self.pending.drain(..=batch_end).collect();
            batches.push(self.next_batch(bytes, batch_records));
        }

        batches
    }

    fn next_batch(&mut self, bytes: Vec<u8>, record_count: usize) -> RecordBatch {
        let sequence = self.next_batch_sequence;
        self.next_batch_sequence += 1;
        RecordBatch {
            sequence,
            bytes,
            record_count,
        }
    }
}

fn nth_record_end(bytes: &[u8], batch_records: usize) -> Option<usize> {
    let mut records = 0_usize;

    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            records += 1;
            if records == batch_records {
                return Some(index);
            }
        }
    }

    None
}

fn count_records(bytes: &[u8]) -> usize {
    let newline_count = bytes.iter().filter(|byte| **byte == b'\n').count();
    newline_count + usize::from(!bytes.is_empty() && !bytes.ends_with(b"\n"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedBatch {
    pub sequence: u64,
    pub bytes: Vec<u8>,
}

pub fn evaluate_batches_ordered<E>(
    mut batches: Vec<RecordBatch>,
    filter_threads: usize,
    mut evaluate: impl FnMut(RecordBatch) -> Result<AcceptedBatch, E>,
) -> Result<Vec<AcceptedBatch>, E> {
    let _bounded_filter_threads = filter_threads.max(1);

    batches.sort_by_key(|batch| batch.sequence);
    let mut accepted = Vec::with_capacity(batches.len());
    for batch in batches {
        accepted.push(evaluate(batch)?);
    }
    accepted.sort_by_key(|batch| batch.sequence);
    Ok(accepted)
}

pub fn write_ordered_batches<W, I>(writer: &mut W, batches: I) -> io::Result<()>
where
    W: Write + ?Sized,
    I: IntoIterator<Item = AcceptedBatch>,
{
    let mut writer = OrderedBatchWriter::new(writer);

    for batch in batches {
        writer.write_batch(batch)?;
    }

    writer.finish()
}

pub struct OrderedBatchWriter<'a, W: Write + ?Sized> {
    writer: &'a mut W,
    next_sequence: u64,
    pending: BTreeMap<u64, Vec<u8>>,
}

impl<'a, W: Write + ?Sized> OrderedBatchWriter<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        Self {
            writer,
            next_sequence: 0,
            pending: BTreeMap::new(),
        }
    }

    pub fn write_batch(&mut self, batch: AcceptedBatch) -> io::Result<()> {
        if batch.sequence < self.next_sequence
            || self.pending.insert(batch.sequence, batch.bytes).is_some()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "duplicate accepted batch sequence",
            ));
        }

        while let Some(bytes) = self.pending.remove(&self.next_sequence) {
            self.writer.write_all(&bytes)?;
            self.next_sequence += 1;
        }

        Ok(())
    }

    pub fn finish(self) -> io::Result<()> {
        if self.pending.is_empty() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "accepted batch sequence gap",
            ))
        }
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

    #[test]
    fn ordered_batch_writer_flushes_incrementally() {
        let mut output = Vec::new();
        let mut writer = OrderedBatchWriter::new(&mut output);

        writer
            .write_batch(AcceptedBatch {
                sequence: 0,
                bytes: b"first\n".to_vec(),
            })
            .unwrap();
        writer
            .write_batch(AcceptedBatch {
                sequence: 1,
                bytes: b"second\n".to_vec(),
            })
            .unwrap();
        writer.finish().unwrap();

        assert_eq!(output, b"first\nsecond\n");
    }

    #[test]
    fn line_carry_preserves_lines_across_blocks() {
        let mut carry = LineCarry {
            pending: Vec::new(),
            next_batch_sequence: 0,
        };

        let first = carry.push_block(
            DecodedBlock {
                block_sequence: 0,
                virtual_offset: 0,
                bytes: b"a\nb".to_vec(),
            },
            2,
        );
        let second = carry.push_block(
            DecodedBlock {
                block_sequence: 1,
                virtual_offset: 3,
                bytes: b"\nc\n".to_vec(),
            },
            2,
        );
        let final_batches = carry.finish();

        assert!(first.is_empty());
        assert_eq!(
            second,
            vec![RecordBatch {
                sequence: 0,
                bytes: b"a\nb\n".to_vec(),
                record_count: 2,
            }]
        );
        assert_eq!(
            final_batches,
            vec![RecordBatch {
                sequence: 1,
                bytes: b"c\n".to_vec(),
                record_count: 1,
            }]
        );
    }

    #[test]
    fn ordered_evaluator_returns_accepted_batches_by_sequence() {
        let batches = vec![
            RecordBatch {
                sequence: 1,
                bytes: b"second\n".to_vec(),
                record_count: 1,
            },
            RecordBatch {
                sequence: 0,
                bytes: b"first\n".to_vec(),
                record_count: 1,
            },
        ];

        let accepted = evaluate_batches_ordered(batches, 4, |batch| {
            Ok::<_, std::convert::Infallible>(AcceptedBatch {
                sequence: batch.sequence,
                bytes: batch.bytes,
            })
        })
        .unwrap();

        assert_eq!(
            accepted,
            vec![
                AcceptedBatch {
                    sequence: 0,
                    bytes: b"first\n".to_vec(),
                },
                AcceptedBatch {
                    sequence: 1,
                    bytes: b"second\n".to_vec(),
                },
            ]
        );
    }
}
