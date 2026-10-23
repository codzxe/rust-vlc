# Speed-parity benchmark (decode-to-null)

corpus: `/tmp/corpus-w7b` | files: 8 | best-of-1 | runners: ffmpeg, lumen

| class              | n                  | runner             | med wall           | p95 wall           | med fps            | med realtime       | ratio vs ffmpeg    |
|--------------------|--------------------|--------------------|--------------------|--------------------|--------------------|--------------------|--------------------|
| avi-av             | 8                  | ffmpeg             | 244.3 ms           | 627.5 ms           | 286.1              | 21.73x             | -                  |
| avi-av             | 8                  | lumen              | 182.9 ms           | 668.3 ms           | 349.1              | 26.16x             | 1.34x              |
