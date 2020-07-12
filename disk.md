When benchmarked as a striped set of 8 SSD drives an ien.24xlarge achieved a sustained write speed of

7380957 KiB / s

at that speed it will take 3.889877423754128 seconds to save 28711018 KiB

fio --directory=/mnt/iops --name fio_test_file --direct=1 --rw=randwrite --bs=64k --size=1G --numjobs=16 --time_based --runtime=180 --group_reporting --norandommap



sudo nvme list | grep Instance | cut -f1 -d' '


Starting 16 processes
Jobs: 16 (f=16): [w(16)] [100.0% done] [0KB/571.7MB/0KB /s] [0/9137/0 iops] [eta 00m:00s]
fio_test_file: (groupid=0, jobs=16): err= 0: pid=3469: Sat May 23 01:26:06 2020
  write: io=103145MB, bw=586,774 KB/s, iops=9168, runt=180002msec
    clat (usec): min=598, max=37518, avg=1742.02, stdev=732.60
     lat (usec): min=602, max=37520, avg=1744.37, stdev=732.58
    clat percentiles (usec):
     |  1.00th=[  812],  5.00th=[  940], 10.00th=[ 1012], 20.00th=[ 1112],
     | 30.00th=[ 1224], 40.00th=[ 1416], 50.00th=[ 1608], 60.00th=[ 1800],
     | 70.00th=[ 2008], 80.00th=[ 2256], 90.00th=[ 2704], 95.00th=[ 3056],
     | 99.00th=[ 3792], 99.50th=[ 4128], 99.90th=[ 5280], 99.95th=[ 6560],
     | 99.99th=[15808]
    lat (usec) : 750=0.39%, 1000=8.54%
    lat (msec) : 2=60.98%, 4=29.43%, 10=0.63%, 20=0.01%, 50=0.01%
  cpu          : usr=0.32%, sys=0.65%, ctx=1651555, majf=0, minf=147
  IO depths    : 1=100.0%, 2=0.0%, 4=0.0%, 8=0.0%, 16=0.0%, 32=0.0%, >=64=0.0%
     submit    : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     complete  : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     issued    : total=r=0/w=1650319/d=0, short=r=0/w=0/d=0, drop=r=0/w=0/d=0
     latency   : target=0, window=0, percentile=100.00%, depth=1

Run status group 0 (all jobs):
  WRITE: io=103145MB, aggrb=586773KB/s, minb=586773KB/s, maxb=586773KB/s, mint=180002msec, maxt=180002msec

Disk stats (read/write):
    md0: ios=0/1697041, merge=0/0, ticks=0/0, in_queue=0, util=0.00%, aggrios=0/275777, aggrmerge=0/7137, aggrticks=0/476592, aggrin_queue=370025, aggrutil=82.66%
  nvme1n1: ios=0/275297, merge=0/7075, ticks=0/474260, in_queue=365564, util=81.93%
  nvme2n1: ios=0/276085, merge=0/7281, ticks=0/472848, in_queue=366736, util=81.65%
  nvme3n1: ios=0/276253, merge=0/6967, ticks=0/483876, in_queue=375920, util=82.66%
  nvme4n1: ios=0/275355, merge=0/7142, ticks=0/474264, in_queue=369380, util=81.55%
  nvme5n1: ios=0/276180, merge=0/7241, ticks=0/482284, in_queue=376636, util=82.34%
  nvme6n1: ios=0/275496, merge=0/7117, ticks=0/472024, in_queue=365916, util=81.86%


```
# fio --directory=/home/ec2-user/iops --name fio_test_file --direct=1 --rw=randwrite --bs=64k --size=256M --numjobs=16 --time_based --runtime=180 --group_reporting
fio_test_file: (g=0): rw=randwrite, bs=64K-64K/64K-64K/64K-64K, ioengine=psync, iodepth=1
...
fio-2.14
Starting 16 processes
Jobs: 16 (f=16): [w(16)] [100.0% done] [0KB/128.2MB/0KB /s] [0/2050/0 iops] [eta 00m:00s]
fio_test_file: (groupid=0, jobs=16): err= 0: pid=3608: Sat May 23 01:38:30 2020
  write: io=23187MB, bw=131901KB/s, iops=2060, runt=180008msec
    clat (usec): min=908, max=34422, avg=7760.25, stdev=645.96
     lat (usec): min=910, max=34424, avg=7762.50, stdev=645.91
    clat percentiles (usec):
     |  1.00th=[ 3600],  5.00th=[ 7520], 10.00th=[ 7648], 20.00th=[ 7712],
     | 30.00th=[ 7776], 40.00th=[ 7776], 50.00th=[ 7776], 60.00th=[ 7840],
     | 70.00th=[ 7840], 80.00th=[ 7840], 90.00th=[ 7968], 95.00th=[ 8096],
     | 99.00th=[ 8256], 99.50th=[ 9152], 99.90th=[12992], 99.95th=[14144],
     | 99.99th=[17024]
    lat (usec) : 1000=0.01%
    lat (msec) : 2=0.02%, 4=1.02%, 10=98.57%, 20=0.39%, 50=0.01%
  cpu          : usr=0.06%, sys=0.14%, ctx=371176, majf=0, minf=161
  IO depths    : 1=100.0%, 2=0.0%, 4=0.0%, 8=0.0%, 16=0.0%, 32=0.0%, >=64=0.0%
     submit    : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     complete  : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     issued    : total=r=0/w=370989/d=0, short=r=0/w=0/d=0, drop=r=0/w=0/d=0
     latency   : target=0, window=0, percentile=100.00%, depth=1

Run status group 0 (all jobs):
  WRITE: io=23187MB, aggrb=131901KB/s, minb=131901KB/s, maxb=131901KB/s, mint=180008msec, maxt=180008msec

Disk stats (read/write):
  nvme0n1: ios=0/371002, merge=0/11, ticks=0/2872596, in_queue=2699308, util=99.93%
```



```
# fio --directory=/home/ec2-user/iops --name fio_test_file --direct=1 --rw=randwrite --bs=64k --size=256M --numjobs=16 --t
ime_based --runtime=180 --group_reporting --iodepth=8
fio_test_file: (g=0): rw=randwrite, bs=64K-64K/64K-64K/64K-64K, ioengine=psync, iodepth=8
...
fio-2.14
Starting 16 processes
Jobs: 16 (f=16): [w(16)] [100.0% done] [0KB/128.2MB/0KB /s] [0/2050/0 iops] [eta 00m:00s]
fio_test_file: (groupid=0, jobs=16): err= 0: pid=3642: Sat May 23 01:50:29 2020
  write: io=23179MB, bw=131856KB/s, iops=2060, runt=180007msec
    clat (usec): min=828, max=34124, avg=7762.61, stdev=763.38
     lat (usec): min=831, max=34125, avg=7764.95, stdev=763.34
    clat percentiles (usec):
     |  1.00th=[ 2992],  5.00th=[ 7520], 10.00th=[ 7648], 20.00th=[ 7712],
     | 30.00th=[ 7776], 40.00th=[ 7776], 50.00th=[ 7776], 60.00th=[ 7840],
     | 70.00th=[ 7840], 80.00th=[ 7904], 90.00th=[ 7968], 95.00th=[ 8096],
     | 99.00th=[ 8384], 99.50th=[10304], 99.90th=[13632], 99.95th=[15040],
     | 99.99th=[25728]
    lat (usec) : 1000=0.01%
    lat (msec) : 2=0.02%, 4=1.18%, 10=98.25%, 20=0.51%, 50=0.03%
  cpu          : usr=0.07%, sys=0.11%, ctx=370890, majf=0, minf=147
  IO depths    : 1=100.0%, 2=0.0%, 4=0.0%, 8=0.0%, 16=0.0%, 32=0.0%, >=64=0.0%
     submit    : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     complete  : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     issued    : total=r=0/w=370860/d=0, short=r=0/w=0/d=0, drop=r=0/w=0/d=0
     latency   : target=0, window=0, percentile=100.00%, depth=8

Run status group 0 (all jobs):
  WRITE: io=23179MB, aggrb=131856KB/s, minb=131856KB/s, maxb=131856KB/s, mint=180007msec, maxt=180007msec

Disk stats (read/write):
  nvme0n1: ios=0/370505, merge=0/6, ticks=0/2868712, in_queue=2695260, util=99.93%
```
```
(base) [ec2-user@ip-10-1-1-235 iops]$ fio --directory=/mnt/iops --name fio_test_file --direct=1 --rw=randwrite --bs=64k --size=1G --numjobs=32 --time_based --
runtime=180 --group_reporting --iodepth=8
fio_test_file: (g=0): rw=randwrite, bs=64K-64K/64K-64K/64K-64K, ioengine=psync, iodepth=8
...
fio-2.14
Starting 32 processes
^Cbs: 32 (f=32): [w(32)] [63.5% done] [0KB/571.0MB/0KB /s] [0/9136/0 iops] [eta 01m:06s]
fio: terminating on signal 2

fio_test_file: (groupid=0, jobs=32): err= 0: pid=3871: Sat May 23 02:04:36 2020
  write: io=65794MB, bw=589622KB/s, iops=9212, runt=114265msec
    clat (usec): min=641, max=23703, avg=3469.79, stdev=2096.66
     lat (usec): min=644, max=23704, avg=3472.11, stdev=2096.64
    clat percentiles (usec):
     |  1.00th=[  900],  5.00th=[ 1096], 10.00th=[ 1224], 20.00th=[ 1672],
     | 30.00th=[ 2040], 40.00th=[ 2448], 50.00th=[ 2928], 60.00th=[ 3504],
     | 70.00th=[ 4192], 80.00th=[ 5088], 90.00th=[ 6496], 95.00th=[ 7712],
     | 99.00th=[ 9792], 99.50th=[10688], 99.90th=[12096], 99.95th=[12608],
     | 99.99th=[13888]
    lat (usec) : 750=0.10%, 1000=2.15%
    lat (msec) : 2=26.57%, 4=38.72%, 10=31.58%, 20=0.88%, 50=0.01%
  cpu          : usr=0.17%, sys=0.31%, ctx=1053806, majf=0, minf=294
  IO depths    : 1=100.0%, 2=0.0%, 4=0.0%, 8=0.0%, 16=0.0%, 32=0.0%, >=64=0.0%
     submit    : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     complete  : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     issued    : total=r=0/w=1052705/d=0, short=r=0/w=0/d=0, drop=r=0/w=0/d=0
     latency   : target=0, window=0, percentile=100.00%, depth=8

Run status group 0 (all jobs):
  WRITE: io=65794MB, aggrb=589621KB/s, minb=589621KB/s, maxb=589621KB/s, mint=114265msec, maxt=114265msec

Disk stats (read/write):
    md0: ios=0/1052032, merge=0/0, ticks=0/0, in_queue=0, util=0.00%, aggrios=0/175458, aggrmerge=0/10, aggrticks=0/605783, aggrin_queue=534891, aggrutil=93.8
2%
  nvme3n1: ios=0/175476, merge=0/12, ticks=0/591320, in_queue=519672, util=92.62%
  nvme6n1: ios=0/175418, merge=0/9, ticks=0/583572, in_queue=512648, util=92.68%
  nvme2n1: ios=0/175454, merge=0/9, ticks=0/600292, in_queue=530088, util=92.58%
  nvme5n1: ios=0/175442, merge=0/9, ticks=0/666780, in_queue=596152, util=93.82%
  nvme1n1: ios=0/175505, merge=0/15, ticks=0/570780, in_queue=498604, util=92.27%
  nvme4n1: ios=0/175457, merge=0/9, ticks=0/621956, in_queue=552184, util=93.36%
```

# Single NVMe drive experiments

The SSD performance of nvme drives is listed in the AWS documentation by tables

- 100% Random Read IOPS
- Write IOPS

where 

> If you use a Linux AMI with kernel version 4.4 or later and use all the SSD-based instance store volumes available to your instance,
> you get the IOPS (4,096 byte block size) performance listed in the following table (at queue depth saturation)

Therefore our estimate of speed for random writes to a single drive should be

listed speed * 4 / 1024 / num drives

assuming 4KB writes (which is not particularly accurate but will do)

### Raw nvme disk speed (m5d.8xlarge)

The raw write speed estimate for an m5d.8large (which has 2 drives) is 233,333 which estimates to

233,333 x 4k / 1024k / 2 = 456 MiB/s

So our

526269 KB/s = 501 MiB/s

is in the right ballpark, possibly explained by our large block size

```
# fio --directory=/mnt/iops --name fio_test_file --direct=1 --rw=randwrite --bs=4M --size=1G --numjobs=32 --time_based --runtime=60 --group_reporting
fio_test_file: (g=0): rw=randwrite, bs=4M-4M/4M-4M/4M-4M, ioengine=psync, iodepth=1
...
fio-2.14
Starting 32 processes
Jobs: 32 (f=32): [w(32)] [100.0% done] [0KB/484.0MB/0KB /s] [0/121/0 iops] [eta 00m:00s]
fio_test_file: (groupid=0, jobs=32): err= 0: pid=4876: Sat May 23 05:02:58 2020
  write: io=30944MB, bw=526269KB/s, iops=128, runt= 60210msec
    clat (msec): min=7, max=495, avg=248.46, stdev=69.17
     lat (msec): min=7, max=495, avg=248.67, stdev=69.17
    clat percentiles (msec):
     |  1.00th=[   97],  5.00th=[  196], 10.00th=[  233], 20.00th=[  233],
     | 30.00th=[  233], 40.00th=[  239], 50.00th=[  239], 60.00th=[  239],
     | 70.00th=[  239], 80.00th=[  239], 90.00th=[  247], 95.00th=[  465],
     | 99.00th=[  478], 99.50th=[  482], 99.90th=[  494], 99.95th=[  494],
     | 99.99th=[  494]
    lat (msec) : 10=0.04%, 100=2.40%, 250=88.44%, 500=9.11%
  cpu          : usr=0.08%, sys=0.07%, ctx=8116, majf=0, minf=261
  IO depths    : 1=100.0%, 2=0.0%, 4=0.0%, 8=0.0%, 16=0.0%, 32=0.0%, >=64=0.0%
     submit    : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     complete  : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     issued    : total=r=0/w=7736/d=0, short=r=0/w=0/d=0, drop=r=0/w=0/d=0
     latency   : target=0, window=0, percentile=100.00%, depth=1

Run status group 0 (all jobs):
  WRITE: io=30944MB, aggrb=526268KB/s, minb=526268KB/s, maxb=526268KB/s, mint=60210msec, maxt=60210msec

Disk stats (read/write):
  nvme7n1: ios=0/247329, merge=0/34, ticks=0/46010024, in_queue=46008972, util=99.82%
```


### Raw nvme disk speed (m5d.4xlarge)

The raw write speed estimate for an m5d.4large (which has 2 drives) is 114,000 which estimates to

114,000 x 4k / 1024k / 2 = 222 MiB/s

So our

250MB/s

seems ok

```
# fio --directory=/mnt/iops --name fio_test_file --direct=1 --rw=randwrite --bs=64k --size=1G --numjobs=32 --time_based --runtime=180 --group_reporting --iodepth=8
fio_test_file: (g=0): rw=randwrite, bs=64K-64K/64K-64K/64K-64K, ioengine=psync, iodepth=8
...
fio-2.14
Starting 32 processes
Jobs: 32 (f=32): [w(32)] [100.0% done] [0KB/251.4MB/0KB /s] [0/4022/0 iops] [eta 00m:00s]
fio_test_file: (groupid=0, jobs=32): err= 0: pid=5038: Sat May 23 03:25:23 2020
  write: io=44847MB, bw=255118KB/s, iops=3986, runt=180008msec
    clat (usec): min=98, max=32393, avg=8025.42, stdev=1156.21
     lat (usec): min=101, max=32393, avg=8026.68, stdev=1156.21
    clat percentiles (usec):
     |  1.00th=[ 7968],  5.00th=[ 7968], 10.00th=[ 7968], 20.00th=[ 7968],
     | 30.00th=[ 7968], 40.00th=[ 7968], 50.00th=[ 7968], 60.00th=[ 7968],
     | 70.00th=[ 7968], 80.00th=[ 7968], 90.00th=[ 7968], 95.00th=[ 7968],
     | 99.00th=[15936], 99.50th=[15936], 99.90th=[17024], 99.95th=[20864],
     | 99.99th=[27264]
    lat (usec) : 100=0.01%, 250=0.01%, 500=0.01%, 750=0.01%, 1000=0.01%
    lat (msec) : 2=0.58%, 4=0.17%, 10=97.81%, 20=1.38%, 50=0.06%
  cpu          : usr=0.04%, sys=0.15%, ctx=719252, majf=0, minf=268
  IO depths    : 1=100.0%, 2=0.0%, 4=0.0%, 8=0.0%, 16=0.0%, 32=0.0%, >=64=0.0%
     submit    : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     complete  : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     issued    : total=r=0/w=717550/d=0, short=r=0/w=0/d=0, drop=r=0/w=0/d=0
     latency   : target=0, window=0, percentile=100.00%, depth=8

Run status group 0 (all jobs):
  WRITE: io=44847MB, aggrb=255117KB/s, minb=255117KB/s, maxb=255117KB/s, mint=180008msec, maxt=180008msec

Disk stats (read/write):
  nvme7n1: ios=0/723239, merge=0/26096, ticks=0/5794628, in_queue=5628196, util=99.94%
```



# Raw IOPS experiments (single EBS volume)

For EBS backed by an SSD drive (default):

> When small I/O operations are physically contiguous, Amazon EBS attempts to merge them into
   a single I/O operation up to the maximum size. For example, for SSD volumes, 
   a single 1,024 KiB I/O operation counts as 4 operations (1,024÷256=4), while
   8 contiguous I/O operations at 32 KiB each count as 1 operation (8×32=256). 
   However, 8 random I/O operations at 32 KiB each count as 8 operations. 
   Each I/O operation under 32 KiB counts as 1 operation.

Therefore a simple estimate of random write performance is if we assume the worst case of
all writes counting as 32 KiB single operations.

IOPS x 32 / 1024 = MiB/s

We should be able to achieve this as the worst case - and then get a small increase in performance
if we can maximise the number of IOPS that count as up to 256 KiB operations.

### Single EBS io1 of 300GB @ 15000iops (md5.8xlarge)

Our expectation is we will get 15000x32/1024 = 468.75 MiB/s

If we force our benchmark to max block size of 32 KiB - we see we can match the expected IOPS
performance exactly.

```
fio_test_file: (g=0): rw=randwrite, bs=32K-32K/32K-32K/32K-32K, ioengine=psync, iodepth=1
WRITE: io=28584MB, aggrb=487812KB/s, minb=487812KB/s, maxb=487812KB/s, mint=60003msec, maxt=60003msec
```
487812 KB = 465 MiB/s

```
fio_test_file: (g=0): rw=randwrite, bs=128K-128K/128K-128K/128K-128K, ioengine=psync, iodepth=1
WRITE: io=30542MB, aggrb=521180KB/s, minb=521180KB/s, maxb=521180KB/s, mint=60008msec, maxt=60008msec
```

521180 KB = 497 MiB/s

```
fio_test_file: (g=0): rw=randwrite, bs=256K-256K/256K-256K/256K-256K, ioengine=psync, iodepth=1
WRITE: io=30530MB, aggrb=520893KB/s, minb=520893KB/s, maxb=520893KB/s, mint=60017msec, maxt=60017msec
```
520893 KB = 497 MiB/s

On really large block sizes I think we will get an in between 32kb->256kb so the IOPS gives us something
in between

508860 KB = 485 MiB/s

```
(base) [ec2-user@ip-10-1-1-83 ~]$ fio --directory=/mnt/iops --name fio_test_file --direct=1 --rw=randwrite --bs=4Mi --kb_base=1024 --size=1G --numjobs=32 --time_based --runtime=60 --group_reporting
fio_test_file: (g=0): rw=randwrite, bs=4M-4M/4M-4M/4M-4M, ioengine=psync, iodepth=1
...
fio-2.14
Starting 32 processes
Jobs: 32 (f=32): [w(32)] [100.0% done] [0KB/504.0MB/0KB /s] [0/126/0 iops] [eta 00m:00s]
fio_test_file: (groupid=0, jobs=32): err= 0: pid=5299: Sat May 23 05:18:21 2020
  write: io=29896MB, bw=508860KB/s, iops=124, runt= 60161msec
    clat (msec): min=3, max=732, avg=257.11, stdev=91.48
     lat (msec): min=4, max=732, avg=257.32, stdev=91.48
    clat percentiles (msec):
     |  1.00th=[   43],  5.00th=[  135], 10.00th=[  159], 20.00th=[  186],
     | 30.00th=[  204], 40.00th=[  225], 50.00th=[  247], 60.00th=[  265],
     | 70.00th=[  293], 80.00th=[  326], 90.00th=[  379], 95.00th=[  424],
     | 99.00th=[  523], 99.50th=[  570], 99.90th=[  635], 99.95th=[  668],
     | 99.99th=[  734]
    lat (msec) : 4=0.01%, 10=0.03%, 50=1.35%, 100=0.72%, 250=49.34%
    lat (msec) : 500=47.15%, 750=1.39%
  cpu          : usr=0.08%, sys=0.10%, ctx=136871, majf=0, minf=266
  IO depths    : 1=100.0%, 2=0.0%, 4=0.0%, 8=0.0%, 16=0.0%, 32=0.0%, >=64=0.0%
     submit    : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     complete  : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     issued    : total=r=0/w=7474/d=0, short=r=0/w=0/d=0, drop=r=0/w=0/d=0
     latency   : target=0, window=0, percentile=100.00%, depth=1

Run status group 0 (all jobs):
  WRITE: io=29896MB, aggrb=508859KB/s, minb=508859KB/s, maxb=508859KB/s, mint=60161msec, maxt=60161msec

Disk stats (read/write):
  nvme5n1: ios=0/122499, merge=0/82, ticks=0/3601572, in_queue=3544152, util=99.84%
```

### Single EBS io1 of 100GB @ 5000iops (md5.8xlarge)

Our expectation is we will get 5000x32/1024 = 156 MiB/s

Actual

```
fio_test_file: (g=0): rw=randwrite, bs=32K-32K/32K-32K/32K-32K, ioengine=psync, iodepth=1
WRITE: io=9485.7MB, aggrb=161869KB/s, minb=161869KB/s, maxb=161869KB/s, mint=60007msec, maxt=60007msec
```

161869 KB = 154 MiB/s

# Striping

### Striped nvme disk speed (m5d.4xlarge)

mdadm --create /dev/md1 --level=0 --chunk=64 --raid-devices=2 /dev/nvme7n1 /dev/nvme8n1

500MB/s

Increased the block size to 4MB to try to get even utilisation across both drives but is still wonky.
It was only once the jobs got to 32 (from 16) that both drives were fully utilised but overall bandwidth
didn't change and just more latency variation.
No matter what the block size I couldn't get much about 500MB/s

```
# fio --directory=/mnt/iops --name fio_test_file --rw=randwrite --bs=4m --size=1G --numjobs=16 --time_based --runtime=60 --group_reporting --direct=1
fio_test_file: (g=0): rw=randwrite, bs=4M-4M/4M-4M/4M-4M, ioengine=psync, iodepth=1
...
fio-2.14
Starting 16 processes
Jobs: 16 (f=16): [w(16)] [100.0% done] [0KB/504.0MB/0KB /s] [0/126/0 iops] [eta 00m:00s]
fio_test_file: (groupid=0, jobs=16): err= 0: pid=5446: Sat May 23 03:49:38 2020
  write: io=30804MB, bw=524611KB/s, iops=128, runt= 60127msec
    clat (msec): min=4, max=246, avg=124.58, stdev=17.22
     lat (msec): min=4, max=246, avg=124.80, stdev=17.22
    clat percentiles (msec):
     |  1.00th=[   32],  5.00th=[  128], 10.00th=[  128], 20.00th=[  128],
     | 30.00th=[  128], 40.00th=[  128], 50.00th=[  128], 60.00th=[  128],
     | 70.00th=[  128], 80.00th=[  128], 90.00th=[  128], 95.00th=[  128],
     | 99.00th=[  131], 99.50th=[  182], 99.90th=[  227], 99.95th=[  235],
     | 99.99th=[  247]
    lat (msec) : 10=0.03%, 20=0.08%, 50=2.57%, 100=1.09%, 250=96.23%
  cpu          : usr=0.18%, sys=0.14%, ctx=7877, majf=0, minf=132
  IO depths    : 1=100.0%, 2=0.0%, 4=0.0%, 8=0.0%, 16=0.0%, 32=0.0%, >=64=0.0%
     submit    : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     complete  : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     issued    : total=r=0/w=7701/d=0, short=r=0/w=0/d=0, drop=r=0/w=0/d=0
     latency   : target=0, window=0, percentile=100.00%, depth=1

Run status group 0 (all jobs):
  WRITE: io=30804MB, aggrb=524611KB/s, minb=524611KB/s, maxb=524611KB/s, mint=60127msec, maxt=60127msec

Disk stats (read/write):
    md1: ios=0/492332, merge=0/0, ticks=0/0, in_queue=0, util=0.00%, aggrios=0/123228, aggrmerge=0/123226, aggrticks=0/7430986, aggrin_queue=7396422, aggrutil=99.76%
  nvme8n1: ios=0/123225, merge=0/123223, ticks=0/219588, in_queue=208384, util=17.27%
  nvme7n1: ios=0/123231, merge=0/123229, ticks=0/14642384, in_queue=14584460, util=99.76%
```

### Striped nvme disk speed (m5d.8xlarge)

mdadm --create /dev/md1 --level=0 --chunk=64 --raid-devices=2 /dev/nvme7n1 /dev/nvme8n1

1011.3MB/s

```
(base) [ec2-user@ip-10-1-1-83 ~]$ fio --directory=/mnt/iops --name fio_test_file --direct=1 --rw=randwrite --bs=4M --size=1G --numjobs=32 --time_based 
--runtime=60 --group_reporting
fio_test_file: (g=0): rw=randwrite, bs=4M-4M/4M-4M/4M-4M, ioengine=psync, iodepth=1
...
fio-2.14
Starting 32 processes
Jobs: 32 (f=32): [w(32)] [100.0% done] [0KB/988.0MB/0KB /s] [0/247/0 iops] [eta 00m:00s]
fio_test_file: (groupid=0, jobs=32): err= 0: pid=7632: Sat May 23 05:49:22 2020
  write: io=60804MB, bw=1011.3MB/s, iops=252, runt= 60126msec
    clat (msec): min=2, max=287, avg=126.22, stdev=31.45
     lat (msec): min=3, max=287, avg=126.44, stdev=31.45
    clat percentiles (msec):
     |  1.00th=[   50],  5.00th=[  115], 10.00th=[  116], 20.00th=[  118],
     | 30.00th=[  120], 40.00th=[  120], 50.00th=[  124], 60.00th=[  124],
     | 70.00th=[  128], 80.00th=[  128], 90.00th=[  128], 95.00th=[  233],
     | 99.00th=[  247], 99.50th=[  247], 99.90th=[  253], 99.95th=[  255],
     | 99.99th=[  289]
    lat (msec) : 4=0.01%, 10=0.02%, 20=0.03%, 50=1.01%, 100=2.91%
    lat (msec) : 250=95.72%, 500=0.30%
  cpu          : usr=0.17%, sys=0.14%, ctx=15590, majf=0, minf=263
  IO depths    : 1=100.0%, 2=0.0%, 4=0.0%, 8=0.0%, 16=0.0%, 32=0.0%, >=64=0.0%
     submit    : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     complete  : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     issued    : total=r=0/w=15201/d=0, short=r=0/w=0/d=0, drop=r=0/w=0/d=0
     latency   : target=0, window=0, percentile=100.00%, depth=1

Run status group 0 (all jobs):
  WRITE: io=60804MB, aggrb=1011.3MB/s, minb=1011.3MB/s, maxb=1011.3MB/s, mint=60126msec, maxt=60126msec

Disk stats (read/write):
    md1: ios=0/982388, merge=0/0, ticks=0/0, in_queue=0, util=0.00%, aggrios=0/246008, aggrmerge=0/246018, aggrticks=0/15293404, aggrin_queue=15252682,
 aggrutil=99.79%
  nvme8n1: ios=0/246014, merge=0/246022, ticks=0/605512, in_queue=582300, util=36.18%
  nvme7n1: ios=0/246002, merge=0/246014, ticks=0/29981296, in_queue=29923064, util=99.79%
```

### Striped EBS io1 @ 6 x 2500iops (m5d.4xlarge)

570MB/s

```
# fio --directory=/mnt2/iops --name fio_test_file --rw=randwrite --bs=4m --size=1G --numjobs=16 --time_based --runtime=60 --group_reporting --direct=1
fio_test_file: (g=0): rw=randwrite, bs=4M-4M/4M-4M/4M-4M, ioengine=psync, iodepth=1
...
fio-2.14
Starting 16 processes
Jobs: 16 (f=16): [w(16)] [100.0% done] [0KB/572.0MB/0KB /s] [0/143/0 iops] [eta 00m:00s]
fio_test_file: (groupid=0, jobs=16): err= 0: pid=5680: Sat May 23 04:03:55 2020
  write: io=34844MB, bw=594255KB/s, iops=145, runt= 60042msec
    clat (msec): min=10, max=181, avg=110.04, stdev=15.17
     lat (msec): min=11, max=182, avg=110.25, stdev=15.17
    clat percentiles (msec):
     |  1.00th=[   23],  5.00th=[   97], 10.00th=[  103], 20.00th=[  108],
     | 30.00th=[  110], 40.00th=[  112], 50.00th=[  113], 60.00th=[  114],
     | 70.00th=[  116], 80.00th=[  118], 90.00th=[  120], 95.00th=[  123],
     | 99.00th=[  135], 99.50th=[  143], 99.90th=[  161], 99.95th=[  172],
     | 99.99th=[  182]
    lat (msec) : 20=0.68%, 50=1.47%, 100=4.73%, 250=93.12%
  cpu          : usr=0.20%, sys=0.16%, ctx=9404, majf=0, minf=129
  IO depths    : 1=100.0%, 2=0.0%, 4=0.0%, 8=0.0%, 16=0.0%, 32=0.0%, >=64=0.0%
     submit    : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     complete  : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     issued    : total=r=0/w=8711/d=0, short=r=0/w=0/d=0, drop=r=0/w=0/d=0
     latency   : target=0, window=0, percentile=100.00%, depth=1

Run status group 0 (all jobs):
  WRITE: io=34844MB, aggrb=594254KB/s, minb=594254KB/s, maxb=594254KB/s, mint=60042msec, maxt=60042msec

Disk stats (read/write):
    md0: ios=0/556716, merge=0/0, ticks=0/0, in_queue=0, util=0.00%, aggrios=0/26350, aggrmerge=0/66574, aggrticks=0/1185014, aggrin_queue=1133052, aggrutil=99.6
1%
  nvme3n1: ios=0/26380, merge=0/66541, ticks=0/588228, in_queue=537236, util=98.57%
  nvme6n1: ios=0/26259, merge=0/66677, ticks=0/2275520, in_queue=2220936, util=99.61%
  nvme2n1: ios=0/26404, merge=0/66532, ticks=0/563940, in_queue=513080, util=96.44%
  nvme5n1: ios=0/26408, merge=0/66503, ticks=0/1728944, in_queue=1676108, util=99.38%
  nvme1n1: ios=0/26381, merge=0/66541, ticks=0/376828, in_queue=326244, util=95.05%
  nvme4n1: ios=0/26271, merge=0/66651, ticks=0/1576628, in_queue=1524708, util=99.49%
```


Usage: mkfs.xfs
/* blocksize */[-b log=n|size=num]
/* metadata */[-m crc=0|1,finobt=0|1,uuid=xxx]
/* data subvol */[-d agcount=n,agsize=n,file,name=xxx,size=num,
    (sunit=value,swidth=value|su=num,sw=num|noalign),
    sectlog=n|sectsize=num
/* force overwrite */[-f]
/* inode size */[-i log=n|perblock=n|size=num,maxpct=n,attr=0|1|2,
    projid32bit=0|1,sparse=0|1]
/* no discard */[-K]
/* log subvol */[-l agnum=n,internal,size=num,logdev=xxx,version=n
    sunit=value|su=num,sectlog=n|sectsize=num,
    lazy-count=0|1]
/* label */[-L label (maximum 12 characters)]
/* naming */[-n log=n|size=num,version=2|ci,ftype=0|1]
/* no-op info only */[-N]
/* prototype file */[-p fname]
/* quiet */[-q]
/* realtime subvol */[-r extsize=num,size=num,rtdev=xxx]
/* sectorsize */[-s log=n|size=num]
/* version */[-V]
devicename
<devicename> is required unless -d name=xxx is given.
<num> is xxx (bytes), xxxs (sectors), xxxb (fs blocks), xxxk (xxx KiB),
      xxxm (xxx MiB), xxxg (xxx GiB), xxxt (xxx TiB) or xxxp (xxx PiB).
<value> is xxx (512 byte blocks).
