# Speed-parity benchmark (decode-to-null)

corpus: `/tmp/corpus-w7b` | files: 8 | best-of-2 | runners: ffmpeg, lumen

| class              | n                  | runner             | med wall           | p95 wall           | med fps            | med realtime       | ratio vs ffmpeg    |
|--------------------|--------------------|--------------------|--------------------|--------------------|--------------------|--------------------|--------------------|
| avi-av             | 8                  | ffmpeg             | 81.7 ms            | 219.2 ms           | 553.7              | 69.45x             | -                  |
| avi-av             | 8                  | lumen              | 73.6 ms            | 482.0 ms           | 619.8              | 50.65x             | 1.11x              |
