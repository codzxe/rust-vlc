# Speed-parity benchmark (decode-to-null)

corpus: `corpus-big` | files: 36 | best-of-2 | runners: ffmpeg

| class              | n                  | runner             | med wall           | p95 wall           | med fps            | med realtime       | ratio vs ffmpeg    |
|--------------------|--------------------|--------------------|--------------------|--------------------|--------------------|--------------------|--------------------|
| audio-only         | 8                  | ffmpeg             | 62.8 ms            | 75.6 ms            | -                  | -                  | -                  |
| avi-video          | 7                  | ffmpeg             | 97.6 ms            | 192.6 ms           | 1311.2             | 43.71x             | -                  |
| cover-audio (low-N) | 1                  | ffmpeg             | 56.8 ms            | 56.8 ms            | 17.6               | -                  | -                  |
| drift-av (low-N)   | 2                  | ffmpeg             | 405.7 ms           | 282.2 ms           | 4563.8             | 190.30x            | -                  |
| flv-video (low-N)  | 1                  | ffmpeg             | 71.2 ms            | 71.2 ms            | 168.4              | 91.24x             | -                  |
| fmp4-video (low-N) | 2                  | ffmpeg             | 73.2 ms            | 65.9 ms            | 2909.5             | 82.48x             | -                  |
| meta-bomb-av (low-N) | 1                  | ffmpeg             | 63.5 ms            | 63.5 ms            | 47.3               | 94.51x             | -                  |
| mkv-video          | 5                  | ffmpeg             | 70.4 ms            | 82.0 ms            | 1427.0             | 100.96x            | -                  |
| multivideo-av (low-N) | 2                  | ffmpeg             | 65.7 ms            | 60.4 ms            | 2190.5             | 73.33x             | -                  |
| muxed-av (low-N)   | 1                  | ffmpeg             | 87.8 ms            | 87.8 ms            | 2699.3             | 107.97x            | -                  |
| ogv-video (low-N)  | 2                  | ffmpeg             | 75.6 ms            | 56.4 ms            | 874.4              | 53.39x             | -                  |
| raw-video (low-N)  | 2                  | ffmpeg             | 77.2 ms            | 76.9 ms            | 3669.1             | -                  | -                  |
| ts-video (low-N)   | 1                  | ffmpeg             | 95.2 ms            | 95.2 ms            | 504.0              | 21.02x             | -                  |
| vfr-av (low-N)     | 1                  | ffmpeg             | 175.9 ms           | 175.9 ms           | 767.7              | 37.18x             | -                  |
