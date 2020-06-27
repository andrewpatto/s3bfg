
# deduce the number of attached nvme 'real' drives
nvme list | grep Instance | cut -f1 -d' ' > /tmp/instance-stores

INSTANCE_STORE_COUNT=$(wc -l /tmp/instance-stores | cut -f1 -d' ')
INSTANCE_STORE_LIST=$(tr '\n' ' ' < /tmp/instance-stores)

EXT4_FEATURES=extent,large_file

case $INSTANCE_STORE_COUNT in
1)
  mkfs.ext4 -v -L scratch -b 4096 -O $EXT4_FEATURES $INSTANCE_STORE_LIST
  ;;
2-9)
  # see https://wiki.archlinux.org/index.php/RAID#Format_the_RAID_filesystem
  # the stride is calculated as 1024KiB(mdadm chunk)/4KiB(ext4 blocksize)=256
  # stripe width = # of physical data disks * stride.
  CHUNK_IN_KIB=1024
  STRIDE=$(($CHUNK_IN_KIB / 4))
  STRIPEWIDTH=$(($INSTANCE_STORE_COUNT * $STRIDE))

  mdadm --create /dev/md0 --chunk=$CHUNK_IN_KIB --level=0 --raid-devices=$INSTANCE_STORE_COUNT $INSTANCE_STORE_LIST
  mkfs.ext4 -v -L scratch -b 4096 -E stride=$STRIDE,stripe-width=$STRIPEWIDTH -O $EXT4_FEATURES /dev/md0

  #sunit = RAID chunk in bytes / 512
  #swidth = sunit * number of drives in RAID array ( - for RAID0, and that minus one for RAID5 )
  # The sunit for a 32kb (or 32768 byte) array would be 32768 / 512 = 64
  # The sunit for a 1024kb (or 1048576 byte) array would be 1048576 / 512 = 2048
  # The command to create such a filesystem for a 32kb chunk size RAID0 array with 2 drives and a 4096 (4kb) block size will look something like this:
  # https://erikugel.wordpress.com/tag/sunit/
  # mkfs.xfs -b size=4096 -d sunit=64,swidth=128 /dev/md0
  # mkfs.xfs -v -L scratch -b size=4096 -d su=2048,sw=$INSTANCE_STORE_COUNT /dev/md0
  ;;
*)
  echo "Unknown instance store count"
  ;;
esac

mount -o nobarrier,discard,data=writeback -L scratch /mnt

chown ec2-user:ec2-user /mnt
