# Speed-parity benchmark (decode-to-null)

corpus: `/tmp/corpus-w7b` | files: 8 | best-of-1 | runners: ffmpeg, lumen

| class              | n                  | runner             | med wall           | p95 wall           | med fps            | med realtime       | ratio vs ffmpeg    |
|--------------------|--------------------|--------------------|--------------------|--------------------|--------------------|--------------------|--------------------|
| avi-video          | 8                  | ffmpeg             | 447.7 ms           | 2021.0 ms          | 166.6              | 9.58x              | -                  |
| avi-video          | 5                  | lumen              | 1507.8 ms          | 1983.2 ms          | 76.9               | 1.39x              | 0.30x              |
