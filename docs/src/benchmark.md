# Benchmark

## Android Benchmark

### measure log method

| Library | Time (ns) | Allocations |
|---------|-----------|-------------|
| logcat  | 2,427     | 7           |
| logan   | 4,726     | 14          |
| ezlog   | 8,404     | 7           |
| xlog    | 12,459    | 7           |

### startup time

startup baseline
```
min 206.4,   median 218.5,   max 251.9
```

startup with ezlog time:
```
min 206.8,   median 216.6,   max 276.6
```