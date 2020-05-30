|                                        | vCPU | ECU      | Memory (GiB) | Instance Storage (GB) | Linux/UNIX Usage | R/W IOPS (4k bs)  | EBS Bandwidth | Network Bandwidth |  S3 Bench | Fio | Us  |
|----------------------------------------|------|----------|--------------|-----------------------|------------------|-------------------|---------------|-------------------|---------- |-----|-----|
|  ----- Designed for compute intensive workloads with the best price performance in Amazon EC2 -------- |
| c5n.large                              | 2    | 10       | 5.25 GiB     | EBS Only              | $0.141 per Hour  | n/a               | Up to 3.5 Gbps| Up to 25 Gbps     | 851       |  |
| c5n.xlarge                             | 4    | 20       | 10.5 GiB     | EBS Only              | $0.282 per Hour  | n/a               | Up to 3.5 Gbps| Up to 25 Gbps     | 1,543     |  |
| c5n.2xlarge                            | 8    | 39       | 21 GiB       | EBS Only              | $0.564 per Hour  | n/a               | Up to 3.5 Gbps| Up to 25 Gbps     | 2,861     |  |
| c5n.4xlarge                            | 16   | 73       | 42 GiB       | EBS Only              | $1.128 per Hour  | n/a               | 3.5 Gbps      | Up to 25 Gbps     | 2,851     | 566 MiB/s (64x512k) 8x40GB | memory (consistent 11 secs, 2654) disk (40 sec best with s3 chunk 128MiB, threads 32) |
| c5n.9xlarge                            | 36   | 139      | 96 GiB       | EBS Only              | $2.538 per Hour  | n/a               | 7 Gbps        | 50 Gbps           | 5,741     |  |
| c5n.18xlarge                           | 72   | 281      | 192 GiB      | EBS Only              | $5.076 per Hour  | n/a               | 14 Gbps	     | 100 Gbps          | 8,003     |  |
| c5n.metal                              | 72   | N/A      | 192 GiB      | EBS Only              | $5.076 per Hour  | n/a               |               |                   |           |  |
|  ------------- |
| c5d.large                              | 2    | 10       | 4 GiB        | 1 x 50 NVMe SSD       | $0.126 per Hour  | 20,000/9,000      |               |               | 942 |
| c5d.xlarge                             | 4    | 20       | 8 GiB        | 1 x 100 NVMe SSD      | $0.252 per Hour  | 40,000/18,000     |               |               | 1,113 |
| c5d.2xlarge                            | 8    | 39       | 16 GiB       | 1 x 200 NVMe SSD      | $0.504 per Hour  | 80,000/37,000     |               |               | |
| c5d.4xlarge                            | 16   | 73       | 32 GiB       | 1 x 400 NVMe SSD      | $1.008 per Hour  | 175,000/75,000    |               |               | 1,156 |
| c5d.9xlarge                            | 36   | 139      | 72 GiB       | 1 x 900 NVMe SSD      | $2.268 per Hour  | 350,000/170,000   |               |               | 1,387 |
| c5d.12xlarge                           | 48   | 188      | 96 GiB       | 2 x 900 NVMe SSD      | $3.024 per Hour  | 700,000/340,000   |               |               | |
| c5d.18xlarge                           | 72   | 281      | 144 GiB      | 2 x 900 NVMe SSD      | $4.536 per Hour  | 700,000/340,000   |               |               | 2,710 |
| c5d.24xlarge                           | 96   | 375      | 192 GiB      | 4 x 900 NVMe SSD      | $6.048 per Hour  | 1,400,000/680,000 |               |               | |
| c5d.metal                              | 96   | 375      | 192 GiB      | 4 x 900 NVMe SSD      | $6.048 per Hour  | 1,400,000/680,000 |               |               | |
|  ------------- |
| m5d.large                              | 2    | 10       | 8 GiB        | 1 x 75 NVMe SSD       | $0.142 per Hour  | 30,000/15,000*   |                | Up to 10 Gbps |    891 |
| m5d.xlarge                             | 4    | 16       | 16 GiB       | 1 x 150 NVMe SSD      | $0.284 per Hour  | 59,000/29,000*   |                | Up to 10 Gbps |  1,114 |
| m5d.2xlarge                            | 8    | 37       | 32 GiB       | 1 x 300 NVMe SSD      | $0.568 per Hour  | 117,000/57,000*  |                | Up to 10 Gbps |  1,136 |
| m5d.4xlarge                            | 16   | 70       | 64 GiB       | 2 x 300 NVMe SSD      | $1.136 per Hour  | 234,000/114,000* |                | Up to 10 Gbps |  1,156 |
| m5d.8xlarge                            | 32   | 128      | 128 GiB      | 2 x 600 NVMe SSD      | $2.272 per Hour  | 466,666/233,333  |                | 10 Gbps       |        | 1018.7MB/s | memory (consistent 24 secs, 1155) disk (consistent 24 secs with s3 128MiB, threads 32)
| m5d.12xlarge                           | 48   | 168      | 192 GiB      | 2 x 900 NVMe SSD      | $3.408 per Hour  | 700,000/340,000   |               | 10 Gbps       |  1,387 |
| m5d.16xlarge                           | 64   | 256      | 256 GiB      | 4 x 600 NVMe SSD      | $4.544 per Hour  | 933,333/466,666   |               | 20 Gbps       |        |
| m5d.24xlarge                           | 96   | 337      | 384 GiB      | 4 x 900 NVMe SSD      | $6.816 per Hour  | 1,400,000/680,000 |               | 25 Gbps       |        |
| m5d.metal                              | 96   | 345      | 384 GiB      | 4 x 900 NVMe SSD      | $6.816 per Hour  | 1,400,000/680,000 |               | 25 Gbps       |  2,673 |
|  ----Dense SSD storage instances for data-intensive workloads --------- |
| i3en.large                             | 2    | 10       | 16 GiB       | 1 x 1250 NVMe SSD     | $0.271 per Hour  | 42,500/32,500     | Up to 4,750 Mbps | Up to 25 Gbps |      |  |
| i3en.xlarge                            | 4    | N/A      | 32 GiB       | 1 x 2500 NVMe SSD     | $0.542 per Hour  | 85,000/65,000     | Up to 4,750 Mbps | Up to 25 Gbps |      |  |
| i3en.2xlarge                           | 8    | 37       | 64 GiB       | 2 x 2500 NVMe SSD     | $1.084 per Hour  | 170,000/130,000   | Up to 4,750 Mbps | Up to 25 Gbps |      |  |
| i3en.3xlarge                           | 12   | N/A      | 96 GiB       | 1 x 7500 NVMe SSD     | $1.626 per Hour  | 250,000/200,000   | Up to 4,750 Mbps | Up to 25 Gbps |      |  |
| i3en.6xlarge                           | 24   | N/A      | 192 GiB      | 2 x 7500 NVMe SSD     | $3.252 per Hour  | 500,000/400,000   | 4,750 Mbps       | 25 Gbps       |      |  |
| i3en.12xlarge                          | 48   | 168      | 384 GiB      | 4 x 7500 NVMe SSD     | $6.504 per Hour  | 1,000,000/800,000 | 9,500 Mbps       | 50 Gbps       |      |  |
| i3en.24xlarge                          | 96   | 337      | 768 GiB      | 8 x 7500 NVMe SSD     | $13.008 per Hour | 2,000,000/1,600,000| 19,000 Mbps      | 100 Gbps      |      | 7558.1MB/s (64x512k) 8xbigGBnvme  | memory (consistent 8 secs, 3800) disk (consistentish 14 secs with s3 128Mib, thread 64)
| i3en.metal                             | 96   | N/A      | 768 GiB      | 8 x 7500 NVMe SSD     | $13.008 per Hour | 2,000,000/1,600,000| 19,000 Mbps      | 100 Gbps      |      |  |
|  ------------- |
|  ------------- |
| General Purpose - Current Generation   |
| a1.medium                              | 1    | N/A      | 2 GiB        | EBS Only              | $0.0333 per Hour | |
| a1.large                               | 2    | N/A      | 4 GiB        | EBS Only              | $0.0666 per Hour | |
| a1.xlarge                              | 4    | N/A      | 8 GiB        | EBS Only              | $0.1332 per Hour | |
| a1.2xlarge                             | 8    | N/A      | 16 GiB       | EBS Only              | $0.2664 per Hour | |
| a1.4xlarge                             | 16   | N/A      | 32 GiB       | EBS Only              | $0.5328 per Hour | |
| a1.metal                               | 16   | N/A      | 32 GiB       | EBS Only              | $0.533 per Hour  | |
| t3.nano                                | 2    | Variable | 0.5 GiB      | EBS Only              | $0.0066 per Hour | |
| t3.micro                               | 2    | Variable | 1 GiB        | EBS Only              | $0.0132 per Hour | |
| t3.small                               | 2    | Variable | 2 GiB        | EBS Only              | $0.0264 per Hour | |
| t3.medium                              | 2    | Variable | 4 GiB        | EBS Only              | $0.0528 per Hour | |
| t3.large                               | 2    | Variable | 8 GiB        | EBS Only              | $0.1056 per Hour | |
| t3.xlarge                              | 4    | Variable | 16 GiB       | EBS Only              | $0.2112 per Hour | |
| t3.2xlarge                             | 8    | Variable | 32 GiB       | EBS Only              | $0.4224 per Hour | |
| t3a.nano                               | 2    | Variable | 0.5 GiB      | EBS Only              | $0.0059 per Hour | |
| t3a.micro                              | 2    | Variable | 1 GiB        | EBS Only              | $0.0119 per Hour | |
| t3a.small                              | 2    | Variable | 2 GiB        | EBS Only              | $0.0238 per Hour | |
| t3a.medium                             | 2    | Variable | 4 GiB        | EBS Only              | $0.0475 per Hour | |
| t3a.large                              | 2    | Variable | 8 GiB        | EBS Only              | $0.095 per Hour  | |
| t3a.xlarge                             | 4    | Variable | 16 GiB       | EBS Only              | $0.1901 per Hour | |
| t3a.2xlarge                            | 8    | Variable | 32 GiB       | EBS Only              | $0.3802 per Hour | |
| t2.nano                                | 1    | Variable | 0.5 GiB      | EBS Only              | $0.0073 per Hour | |
| t2.micro                               | 1    | Variable | 1 GiB        | EBS Only              | $0.0146 per Hour | 46 |
| t2.small                               | 1    | Variable | 2 GiB        | EBS Only              | $0.0292 per Hour | 39 |
| t2.medium                              | 2    | Variable | 4 GiB        | EBS Only              | $0.0584 per Hour | |
| t2.large                               | 2    | Variable | 8 GiB        | EBS Only              | $0.1168 per Hour | |
| t2.xlarge                              | 4    | Variable | 16 GiB       | EBS Only              | $0.2336 per Hour | |
| t2.2xlarge                             | 8    | Variable | 32 GiB       | EBS Only              | $0.4672 per Hour | |
| m5.large                               | 2    | 10       | 8 GiB        | EBS Only              | $0.12 per Hour   | |
| m5.xlarge                              | 4    | 16       | 16 GiB       | EBS Only              | $0.24 per Hour   | |
| m5.2xlarge                             | 8    | 37       | 32 GiB       | EBS Only              | $0.48 per Hour   | |
| m5.4xlarge                             | 16   | 70       | 64 GiB       | EBS Only              | $0.96 per Hour   | |
| m5.8xlarge                             | 32   | 128      | 128 GiB      | EBS Only              | $1.92 per Hour   | |
| m5.12xlarge                            | 48   | 168      | 192 GiB      | EBS Only              | $2.88 per Hour   | |
| m5.16xlarge                            | 64   | 256      | 256 GiB      | EBS Only              | $3.84 per Hour   | |
| m5.24xlarge                            | 96   | 337      | 384 GiB      | EBS Only              | $5.76 per Hour   | |
| m5.metal                               | 96   | 345      | 384 GiB      | EBS Only              | $5.76 per Hour   | |
|  ------------- |
| m5a.large                              | 2    | N/A      | 8 GiB        | EBS Only              | $0.108 per Hour  | |
| m5a.xlarge                             | 4    | N/A      | 16 GiB       | EBS Only              | $0.216 per Hour  | |
| m5a.2xlarge                            | 8    | N/A      | 32 GiB       | EBS Only              | $0.432 per Hour  | |
| m5a.4xlarge                            | 16   | N/A      | 64 GiB       | EBS Only              | $0.864 per Hour  | |
| m5a.8xlarge                            | 32   | N/A      | 128 GiB      | EBS Only              | $1.728 per Hour  | |
| m5a.12xlarge                           | 48   | N/A      | 192 GiB      | EBS Only              | $2.592 per Hour  | |
| m5a.16xlarge                           | 64   | N/A      | 256 GiB      | EBS Only              | $3.456 per Hour  | |
| m5a.24xlarge                           | 96   | N/A      | 384 GiB      | EBS Only              | $5.184 per Hour  | 1,801 |
|  ------------- |
| m5ad.large                             | 2    | N/A      | 8 GiB        | 1 x 75 NVMe SSD       | $0.13 per Hour   | |
| m5ad.xlarge                            | 4    | N/A      | 16 GiB       | 1 x 150 NVMe SSD      | $0.26 per Hour   | |
| m5ad.2xlarge                           | 8    | N/A      | 32 GiB       | 1 x 300 NVMe SSD      | $0.52 per Hour   | |
| m5ad.4xlarge                           | 16   | N/A      | 64 GiB       | 2 x 300 NVMe SSD      | $1.04 per Hour   | |
| m5ad.8xlarge                           | 32   | N/A      | 128 GiB      | 2 x 600 NVMe SSD      | $2.08 per Hour   | |
| m5ad.12xlarge                          | 48   | N/A      | 192 GiB      | 2 x 900 NVMe SSD      | $3.12 per Hour   | |
| m5ad.16xlarge                          | 64   | N/A      | 256 GiB      | 4 x 600 NVMe SSD      | $4.16 per Hour   | |
| m5ad.24xlarge                          | 96   | N/A      | 384 GiB      | 4 x 900 NVMe SSD      | $6.24 per Hour   | 1,706 |
|  ------------- |
| m4.large                               | 2    | 6.5      | 8 GiB        | EBS Only              | $0.125 per Hour  | 53 |
| m4.xlarge                              | 4    | 13       | 16 GiB       | EBS Only              | $0.25 per Hour   | |
| m4.2xlarge                             | 8    | 26       | 32 GiB       | EBS Only              | $0.50 per Hour   | |
| m4.4xlarge                             | 16   | 53.5     | 64 GiB       | EBS Only              | $1.00 per Hour   | |
| m4.10xlarge                            | 40   | 124.5    | 160 GiB      | EBS Only              | $2.50 per Hour   | |
| m4.16xlarge                            | 64   | 188      | 256 GiB      | EBS Only              | $4.00 per Hour   | |
|  ------------- |
| c5.large                               | 2    | 10       | 4 GiB        | EBS Only              | $0.111 per Hour  | |
| c5.xlarge                              | 4    | 20       | 8 GiB        | EBS Only              | $0.222 per Hour  | |
| c5.2xlarge                             | 8    | 39       | 16 GiB       | EBS Only              | $0.444 per Hour  | |
| c5.4xlarge                             | 16   | 73       | 32 GiB       | EBS Only              | $0.888 per Hour  | |
| c5.9xlarge                             | 36   | 139      | 72 GiB       | EBS Only              | $1.998 per Hour  | |
| c5.12xlarge                            | 48   | 188      | 96 GiB       | EBS Only              | $2.664 per Hour  | |
| c5.18xlarge                            | 72   | 281      | 144 GiB      | EBS Only              | $3.996 per Hour  | |
| c5.24xlarge                            | 96   | 375      | 192 GiB      | EBS Only              | $5.328 per Hour  | |
| c5.metal                               | 96   | 375      | 192 GiB      | EBS Only              | $5.328 per Hour  | |
| c4.large                               | 2    | 8        | 3.75 GiB     | EBS Only              | $0.13 per Hour   | |
| c4.xlarge                              | 4    | 16       | 7.5 GiB      | EBS Only              | $0.261 per Hour  | |
| c4.2xlarge                             | 8    | 31       | 15 GiB       | EBS Only              | $0.522 per Hour  | |
| c4.4xlarge                             | 16   | 62       | 30 GiB       | EBS Only              | $1.042 per Hour  | |
| c4.8xlarge                             | 36   | 132      | 60 GiB       | EBS Only              | $2.085 per Hour  | |
| GPU Instances - Current Generation     |
| p3.2xlarge                             | 8    | 31       | 61 GiB       | EBS Only              | $4.234 per Hour  | |
| p3.8xlarge                             | 32   | 97       | 244 GiB      | EBS Only              | $16.936 per Hour | |
| p3.16xlarge                            | 64   | 201      | 488 GiB      | EBS Only              | $33.872 per Hour | |
| p2.xlarge                              | 4    | 16       | 61 GiB       | EBS Only              | $1.542 per Hour  | |
| p2.8xlarge                             | 32   | 97       | 488 GiB      | EBS Only              | $12.336 per Hour | |
| p2.16xlarge                            | 64   | 201      | 732 GiB      | EBS Only              | $24.672 per Hour | |
| g4dn.xlarge                            | 4    | N/A      | 16 GiB       | 125 GB NVMe SSD       | $0.684 per Hour  | |
| g4dn.2xlarge                           | 8    | N/A      | 32 GiB       | 225 GB NVMe SSD       | $0.978 per Hour  | |
| g4dn.4xlarge                           | 16   | N/A      | 64 GiB       | 225 GB NVMe SSD       | $1.566 per Hour  | |
| g4dn.8xlarge                           | 32   | N/A      | 128 GiB      | 900 GB NVMe SSD       | $2.83 per Hour   | |
| g4dn.12xlarge                          | 48   | N/A      | 192 GiB      | 900 GB NVMe SSD       | $5.087 per Hour  | |
| g4dn.16xlarge                          | 64   | N/A      | 256 GiB      | 900 GB NVMe SSD       | $5.659 per Hour  | |
| g3.4xlarge                             | 16   | 58       | 122 GiB      | EBS Only              | $1.754 per Hour  | |
| g3.8xlarge                             | 32   | 97       | 244 GiB      | EBS Only              | $3.508 per Hour  | |
| g3.16xlarge                            | 64   | 201      | 488 GiB      | EBS Only              | $7.016 per Hour  | |
| g3s.xlarge                             | 4    | 13       | 30.5 GiB     | EBS Only              | $1.154 per Hour  | |
| Memory Optimized - Current Generation  |
| x1.16xlarge                            | 64   | 174.5    | 976 GiB      | 1 x 1920 SSD          | $9.671 per Hour  | |
| x1.32xlarge                            | 128  | 349      | 1,952 GiB    | 2 x 1920 SSD          | $19.341 per Hour | |
| x1e.xlarge                             | 4    | 12       | 122 GiB      | 1 x 120 SSD           | $1.209 per Hour  | |
| x1e.2xlarge                            | 8    | 23       | 244 GiB      | 1 x 240 SSD           | $2.418 per Hour  | |
| x1e.4xlarge                            | 16   | 47       | 488 GiB      | 1 x 480 SSD           | $4.836 per Hour  | |
| x1e.8xlarge                            | 32   | 91       | 976 GiB      | 1 x 960 SSD           | $9.672 per Hour  | |
| x1e.16xlarge                           | 64   | 179      | 1,952 GiB    | 1 x 1920 SSD          | $19.344 per Hour | |
| x1e.32xlarge                           | 128  | 340      | 3,904 GiB    | 2 x 1920 SSD          | $38.688 per Hour | |
| r5.large                               | 2    | 10       | 16 GiB       | EBS Only              | $0.151 per Hour  | |
| r5.xlarge                              | 4    | 19       | 32 GiB       | EBS Only              | $0.302 per Hour  | |
| r5.2xlarge                             | 8    | 37       | 64 GiB       | EBS Only              | $0.604 per Hour  | |
| r5.4xlarge                             | 16   | 70       | 128 GiB      | EBS Only              | $1.208 per Hour  | |
| r5.8xlarge                             | 32   | 128      | 256 GiB      | EBS Only              | $2.416 per Hour  | |
| r5.12xlarge                            | 48   | 168      | 384 GiB      | EBS Only              | $3.624 per Hour  | |
| r5.16xlarge                            | 64   | 256      | 512 GiB      | EBS Only              | $4.832 per Hour  | |
| r5.24xlarge                            | 96   | 337      | 768 GiB      | EBS Only              | $7.248 per Hour  | |
| r5.metal                               | 96   | 347      | 768 GiB      | EBS Only              | $7.248 per Hour  | |
| r5a.large                              | 2    | N/A      | 16 GiB       | EBS Only              | $0.136 per Hour  | |
| r5a.xlarge                             | 4    | N/A      | 32 GiB       | EBS Only              | $0.272 per Hour  | |
| r5a.2xlarge                            | 8    | N/A      | 64 GiB       | EBS Only              | $0.544 per Hour  | |
| r5a.4xlarge                            | 16   | N/A      | 128 GiB      | EBS Only              | $1.088 per Hour  | |
| r5a.8xlarge                            | 32   | N/A      | 256 GiB      | EBS Only              | $2.176 per Hour  | |
| r5a.12xlarge                           | 48   | N/A      | 384 GiB      | EBS Only              | $3.264 per Hour  | |
| r5a.16xlarge                           | 64   | N/A      | 512 GiB      | EBS Only              | $4.352 per Hour  | |
| r5a.24xlarge                           | 96   | N/A      | 768 GiB      | EBS Only              | $6.528 per Hour  | |
| r5ad.large                             | 2    | N/A      | 16 GiB       | 1 x 75 NVMe SSD       | $0.159 per Hour  | |
| r5ad.xlarge                            | 4    | N/A      | 32 GiB       | 1 x 150 NVMe SSD      | $0.318 per Hour  | |
| r5ad.2xlarge                           | 8    | N/A      | 64 GiB       | 1 x 300 NVMe SSD      | $0.636 per Hour  | |
| r5ad.4xlarge                           | 16   | N/A      | 128 GiB      | 2 x 300 NVMe SSD      | $1.272 per Hour  | |
| r5ad.8xlarge                           | 32   | N/A      | 256 GiB      | 2 x 600 NVMe SSD      | $2.544 per Hour  | |
| r5ad.12xlarge                          | 48   | N/A      | 384 GiB      | 2 x 900 NVMe SSD      | $3.816 per Hour  | |
| r5ad.16xlarge                          | 64   | N/A      | 512 GiB      | 4 x 600 NVMe SSD      | $5.088 per Hour  | |
| r5ad.24xlarge                          | 96   | N/A      | 768 GiB      | 4 x 900 NVMe SSD      | $7.632 per Hour  | |
| r5d.large                              | 2    | 10       | 16 GiB       | 1 x 75 NVMe SSD       | $0.174 per Hour  | |
| r5d.xlarge                             | 4    | 19       | 32 GiB       | 1 x 150 NVMe SSD      | $0.348 per Hour  | |
| r5d.2xlarge                            | 8    | 37       | 64 GiB       | 1 x 300 NVMe SSD      | $0.696 per Hour  | |
| r5d.4xlarge                            | 16   | 70       | 128 GiB      | 2 x 300 NVMe SSD      | $1.392 per Hour  | |
| r5d.8xlarge                            | 32   | 128      | 256 GiB      | 2 x 600 NVMe SSD      | $2.784 per Hour  | |
| r5d.12xlarge                           | 48   | 168      | 384 GiB      | 2 x 900 NVMe SSD      | $4.176 per Hour  | |
| r5d.16xlarge                           | 64   | 256      | 512 GiB      | 4 x 600 NVMe SSD      | $5.568 per Hour  | |
| r5d.24xlarge                           | 96   | 337      | 768 GiB      | 4 x 900 NVMe SSD      | $8.352 per Hour  | 2,718 |
| r5d.metal                              | 96   | 347      | 768 GiB      | 4 x 900 NVMe SSD      | $8.352 per Hour  | |
| r4.large                               | 2    | 8        | 15.25 GiB    | EBS Only              | $0.1596 per Hour | 541 |
| r4.xlarge                              | 4    | 16       | 30.5 GiB     | EBS Only              | $0.3192 per Hour | |
| r4.2xlarge                             | 8    | 31       | 61 GiB       | EBS Only              | $0.6384 per Hour | |
| r4.4xlarge                             | 16   | 58       | 122 GiB      | EBS Only              | $1.2768 per Hour | |
| r4.8xlarge                             | 32   | 97       | 244 GiB      | EBS Only              | $2.5536 per Hour | |
| r4.16xlarge                            | 64   | 201      | 488 GiB      | EBS Only              | $5.1072 per Hour | |
|  ------------- |
| z1d.large                              | 2    | 12       | 16 GiB       | 1 x 75 NVMe SSD       | $0.226 per Hour  | 1,002 |
| z1d.xlarge                             | 4    | 23       | 32 GiB       | 1 x 150 NVMe SSD      | $0.452 per Hour  | 1,116 |
| z1d.2xlarge                            | 8    | 45       | 64 GiB       | 1 x 300 NVMe SSD      | $0.904 per Hour  | 1,132   |
| z1d.3xlarge                            | 12   | 64       | 96 GiB       | 1 x 450 NVMe SSD      | $1.356 per Hour  | 1,155 |
| z1d.6xlarge                            | 24   | 116      | 192 GiB      | 1 x 900 NVMe SSD      | $2.712 per Hour  | 1,387 |
| z1d.12xlarge                           | 48   | 235      | 384 GiB      | 2 x 900 NVMe SSD      | $5.424 per Hour  | 2,718 |
| z1d.metal                              | 48   | 271      | 384 GiB      | 2 x 900 NVMe SSD      | $5.424 per Hour  | 2,708 |
|  ------------- |
| i3.large                               | 2    | 8        | 15.25 GiB    | 1 x 475 NVMe SSD      | $0.187 per Hour  | i3.4xlarge and smaller	Up to 10 Gbps|
| i3.xlarge                              | 4    | 16       | 30.5 GiB     | 1 x 950 NVMe SSD      | $0.374 per Hour  | i3.4xlarge and smaller	Up to 10 Gbps|
| i3.2xlarge                             | 8    | 31       | 61 GiB       | 1 x 1900 NVMe SSD     | $0.748 per Hour  | 1,143  i3.4xlarge and smaller	Up to 10 Gbps|
| i3.4xlarge                             | 16   | 58       | 122 GiB      | 2 x 1900 NVMe SSD     | $1.496 per Hour  | 1,162  i3.4xlarge	Up to 10 Gbps|
| i3.8xlarge                             | 32   | 97       | 244 GiB      | 4 x 1900 NVMe SSD     | $2.992 per Hour  | 1,400  i3.8xlarge 	10 Gbps|
| i3.16xlarge                            | 64   | 201      | 488 GiB      | 8 x 1900 NVMe SSD     | $5.984 per Hour  | 2,707   |
| d2.xlarge                              | 4    | 14       | 30.5 GiB     | 3 x 2000 HDD          | $0.87 per Hour   | |
| d2.2xlarge                             | 8    | 28       | 61 GiB       | 6 x 2000 HDD          | $1.74 per Hour   | |
| d2.4xlarge                             | 16   | 56       | 122 GiB      | 12 x 2000 HDD         | $3.48 per Hour   | |
| d2.8xlarge                             | 36   | 116      | 244 GiB      | 24 x 2000 HDD         | $6.96 per Hour   | |
