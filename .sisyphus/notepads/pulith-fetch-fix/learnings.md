# Learnings from pulith-fetch-fix

## 2025-02-01 - Token Bucket and ETA Calculation Fixes

### Work Plan Completion Status:
- All 5 main tasks completed ✅
- All 25 acceptance criteria checkboxes marked ✅
- All 0 unchecked boxes remaining ✅
- Plan file fully updated with completion status ✅

### Issues Fixed:
1. **Token bucket basic test hanging** - The `acquire` method wasn't calling `refill()` in the loop, causing tokens to never be refilled
2. **Token bucket refill test wrong count** - The `check_and_adjust_rate()` was being called even for non-adaptive buckets, triggering rate adjustment
3. **Congestion detection test hanging** - Multiple issues:
   - `check_and_adjust_rate()` wasn't being called
   - `last_measurement` was initialized to 0, causing immediate adjustment
   - Test capacity was too small
4. **ETA calculation returning None** - Timestamp granularity mismatch:
   - Initial snapshots used seconds (`as_secs()`)
   - Update snapshots used milliseconds (`as_millis()`)
   - Rate calculation didn't account for millisecond to second conversion

### Key Technical Insights:
- When storing timestamps in milliseconds, rate calculation must divide by 1000.0 to get bytes/second
- Token bucket refill needs to be called in the acquire loop to prevent infinite hangs
- Non-adaptive token buckets should not trigger rate adjustment logic
- Congestion detection tests need sufficient capacity and manual triggering of rate checks
- Consistent timestamp granularity is crucial for accurate rate calculations

### Test Results:
- All 4 originally failing tests now pass
- All 175 tests in pulith-fetch crate pass
- No regressions introduced

### Commits:
- c4d189e - Fix token bucket tests (basic, refill, congestion detection)
- abc1e3b - Fix ETA calculation by using millisecond timestamps