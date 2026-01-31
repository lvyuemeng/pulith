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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_segments_basic() {
        let segments = calculate_segments(100, 4).unwrap();
        assert_eq!(segments.len(), 4);

        // Check that segments cover the entire range
        assert_eq!(segments[0].start, 0);
        assert_eq!(segments[0].end, 25);
        assert_eq!(segments[1].start, 25);
        assert_eq!(segments[1].end, 50);
        assert_eq!(segments[2].start, 50);
        assert_eq!(segments[2].end, 75);
        assert_eq!(segments[3].start, 75);
        assert_eq!(segments[3].end, 100);
    }

    #[test]
    fn test_calculate_segments_with_remainder() {
        let segments = calculate_segments(100, 3).unwrap();
        assert_eq!(segments.len(), 3);

        // 100 / 3 = 33 with remainder 1
        // First segment gets 34, others get 33
        assert_eq!(segments[0].start, 0);
        assert_eq!(segments[0].end, 34);
        assert_eq!(segments[1].start, 34);
        assert_eq!(segments[1].end, 67);
        assert_eq!(segments[2].start, 67);
        assert_eq!(segments[2].end, 100);
    }

    #[test]
    fn test_calculate_segments_single_segment() {
        let segments = calculate_segments(100, 1).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].index, 0);
        assert_eq!(segments[0].start, 0);
        assert_eq!(segments[0].end, 100);
    }

    #[test]
    fn test_calculate_segments_empty_file() {
        let segments = calculate_segments(0, 5).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].index, 0);
        assert_eq!(segments[0].start, 0);
        assert_eq!(segments[0].end, 0);
    }

    #[test]
    fn test_calculate_segments_zero_segments() {
        let result = calculate_segments(100, 0);
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::Error::InvalidState(msg) => {
                assert!(msg.contains("Number of segments must be greater than 0"));
            }
            _ => panic!("Expected InvalidState error"),
        }
    }

    #[test]
    fn test_calculate_segments_large_remainder() {
        let segments = calculate_segments(10, 3).unwrap();
        assert_eq!(segments.len(), 3);

        // 10 / 3 = 3 with remainder 1
        // First segment gets 4, others get 3
        assert_eq!(segments[0].start, 0);
        assert_eq!(segments[0].end, 4);
        assert_eq!(segments[1].start, 4);
        assert_eq!(segments[1].end, 7);
        assert_eq!(segments[2].start, 7);
        assert_eq!(segments[2].end, 10);
    }

    #[test]
    fn test_calculate_segments_more_segments_than_bytes() {
        let segments = calculate_segments(5, 10).unwrap();
        assert_eq!(segments.len(), 10);

        // Each segment should be 0 or 1 byte
        let mut total_size = 0;
        for segment in &segments {
            total_size += segment.end - segment.start;
            assert!(segment.end - segment.start <= 1);
        }
        assert_eq!(total_size, 5);
    }

    #[test]
    fn test_segment_equality() {
        let segment1 = Segment {
            index: 0,
            start: 0,
            end: 10,
        };
        let segment2 = Segment {
            index: 0,
            start: 0,
            end: 10,
        };
        let segment3 = Segment {
            index: 1,
            start: 0,
            end: 10,
        };

        assert_eq!(segment1, segment2);
        assert_ne!(segment1, segment3);
    }

    #[test]
    fn test_segment_debug() {
        let segment = Segment {
            index: 1,
            start: 10,
            end: 20,
        };
        let debug_str = format!("{:?}", segment);
        assert!(debug_str.contains("Segment"));
        assert!(debug_str.contains("index: 1"));
        assert!(debug_str.contains("start: 10"));
        assert!(debug_str.contains("end: 20"));
    }

    #[test]
    fn test_segment_clone() {
        let segment = Segment {
            index: 2,
            start: 20,
            end: 30,
        };
        let cloned = segment.clone();
        assert_eq!(segment, cloned);
    }
}
