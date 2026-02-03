## Issues Found

### Token Bucket Basic Test Issue
- **Problem**: The test `test_token_bucket_basic` fails with assertion `elapsed >= Duration::from_millis(450)`
- **Root Cause**: The `refill()` method is called at the beginning of each loop iteration in `acquire()`, which immediately adds tokens based on elapsed time since the last refill
- **Impact**: When the bucket is empty and we try to acquire 25 bytes, the elapsed time since the last refill might be very small (microseconds), but it still adds some tokens, allowing faster acquisition than the expected 500ms

### Proposed Solution
- Modify the `acquire()` method to track when tokens were last consumed
- Only refill based on the actual time elapsed since tokens were consumed, not since the last refill call
- Ensure proper timing by calculating wait times accurately based on the deficit