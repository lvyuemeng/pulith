/// A segment of a file to be downloaded in parallel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// Segment index (0-based)
    pub index: u32,
    /// Starting byte offset
    pub start: u64,
    /// Ending byte offset (exclusive)
    pub end: u64,
}

/// Calculate segments for parallel download of a file.
///
/// # Arguments
///
/// * `file_size` - Total size of the file to be segmented
/// * `num_segments` - Number of segments to create
///
/// # Returns
///
/// A vector of `Segment` structs defining the byte ranges for each segment.
///
/// # Panics
///
/// Panics if `num_segments` is 0.
pub fn calculate_segments(
    file_size: u64,
    num_segments: u32,
) -> Result<Vec<Segment>, crate::error::Error> {
    if num_segments == 0 {
        return Err(crate::error::Error::InvalidState(
            "Number of segments must be greater than 0".into(),
        ));
    }

    if file_size == 0 {
        return Ok(vec![Segment {
            index: 0,
            start: 0,
            end: 0,
        }]);
    }

    let num_segments = num_segments as u64;
    let segment_size = file_size / num_segments;
    let remainder = file_size % num_segments;

    let mut segments = Vec::with_capacity(num_segments as usize);
    let mut current_offset = 0;

    for i in 0..num_segments {
        let start = current_offset;
        // Add an extra byte to the first 'remainder' segments to distribute the remainder
        let size = if i < remainder {
            segment_size + 1
        } else {
            segment_size
        };
        let end = start + size;
        current_offset = end;

        segments.push(Segment {
            index: i as u32,
            start,
            end,
        });
    }

    Ok(segments)
}
