# Speed-parity benchmark (decode-to-null)

corpus: `/tmp/corpus-7d-speed` | files: 2 | best-of-2 | runners: ffmpeg, lumen

| class              | n                  | runner             | med wall           | p95 wall           | med fps            | med realtime       | ratio vs ffmpeg    |
|--------------------|--------------------|--------------------|--------------------|--------------------|--------------------|--------------------|--------------------|
| avi-video (low-N)  | 2                  | ffmpeg             | 2492.1 ms          | 1219.9 ms          | 232.2              | 1.65x              | -                  |
| avi-video (low-N)  | 2                  | lumen              | 7326.8 ms          | 1779.0 ms          | 89.3               | 0.84x              | 0.34x              |
