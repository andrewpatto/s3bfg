
By default connections to S3 servers are persistent (as per HTTP 1.1).
Note: the addition of *any* keep alive header by the client (as per HTTP 1.0) apparently tells the S3 server to
*not* make the connection persistent - no matter what the value is set to. So if you add a Connection: keep-alive
to your headers the connection *will* be closed immediately after your first operation.

The S3 persistent connections have an extremely short inactivity timeout.
Experimentation puts this timeout at approximately 5 seconds.

Whilst a single connection is capable of transferring many gigabytes of data, each single persistent connection
can perform no more than 100 HTTP GET operations before the connection will be closed. This means that
the size of the objects being fetched (in the case of partial files) will determine how much reuse can be
made out of connections.

The maximum transfer rate over a single S3 connection is 95-100 MiB/s.
(benchmarked on a m5d.8xlarge (which has guaranteed 1000+ MiB/s ethernet)). You can see this on an amply provisioned EC2 instance with
a simple curl command

`curl https://gos-test-cases-public.s3.ap-southeast-2.amazonaws.com/bigfile.1 > bigfile.1`

```
$ curl https://gos-test-cases-public.s3.ap-southeast-2.amazonaws.com/bigfile.1 > bigfile.1
  % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current
                                 Dload  Upload   Total   Spent    Left  Speed
100 1083M  100 1083M    0     0  94.2M      0  0:00:11  0:00:11 --:--:-- 98.1M
```

Note was are not using the aws s3 CLI at this point as it is multi threaded.




ssm-user@ip-172-31-8-71:~$ s3-benchmark -payloads-min 19 -payloads-max 19 -threads-max 48 -threads-min 36 -samples 100

--- SETUP --------------------------------------------------------------------------------------------------------------------

Uploading 256 MB objects
 100% |████████████████████████████████████████|  [56s:0s]

--- BENCHMARK ----------------------------------------------------------------------------------------------------------------

Download performance with 256 MB objects (m5dn.8xlarge)
                           +-------------------------------------------------------------------------------------------------+
                           |            Time to First Byte (ms)             |            Time to Last Byte (ms)              |
+---------+----------------+------------------------------------------------+------------------------------------------------+
| Threads |     Throughput |  avg   min   p25   p50   p75   p90   p99   max |  avg   min   p25   p50   p75   p90   p99   max |
+---------+----------------+------------------------------------------------+------------------------------------------------+
|      36 |    2566.0 MB/s |   73    11    14    18   132   132   568   568 | 3070  2686  2690  2868  3214  3871  4392  4944 |
|      37 |    2963.4 MB/s |   87    12    15    23    63   259   271   685 | 2886  2686  2687  2695  2889  3277  4705  5026 |
|      38 |    3000.5 MB/s |   40    11    14    20    42   136   214   215 | 2848  2686  2688  2705  2903  3098  4057  4136 |
|      39 |    3002.6 MB/s |   38    10    14    17    49    62   162   283 | 2886  2686  2687  2736  2966  3356  4246  4447 |
|      40 |    3144.7 MB/s |   60    10    14    39   122   123   208   208 | 2823  2686  2687  2690  2765  3194  4224  4395 |
|      41 |    2789.8 MB/s |   41    11    14    18    50    72   162   231 | 3046  2686  2691  2797  3167  3919  5186  5225 |
|      42 |    3157.6 MB/s |   64    10    14    23   110   124   457   457 | 2860  2686  2687  2691  2747  3388  4096  4389 |
|      43 |    2834.0 MB/s |   88    12    17    25   107   323   323   323 | 3049  2686  2694  2863  3223  3746  4398  5253 |
|      44 |    3144.8 MB/s |   52    11    15    35    58   133   134   134 | 2969  2686  2688  2732  3037  3541  4741  5201 |
|      45 |    3113.2 MB/s |   62    12    14    19   100   210   210   242 | 3087  2686  2692  2805  3268  3772  4895  5144 |
|      46 |    3140.8 MB/s |   54    11    14    26    45   186   424   426 | 3029  2686  2687  2754  3192  3761  4969  5446 |
|      47 |    3149.6 MB/s |   49    10    14    19    44   134   248   248 | 3044  2686  2687  2692  3016  3822  5746  6057 |
|      48 |    2977.9 MB/s |   66    12    16    28   133   133   388   388 | 3248  2686  2739  2922  3362  4424  5765  6321 |
+---------+----------------+------------------------------------------------+------------------------------------------------+
