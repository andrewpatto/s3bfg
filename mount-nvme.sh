nvme list | grep Instance | cut -f1 -d' ' > /tmp/instance-stores

INSTANCE_STORE_COUNT=$(wc -l /tmp/instance-stores | cut -f1 -d' ')
INSTANCE_STORE_LIST=$(tr '\n' ' ' < /tmp/instance-stores)

mdadm --create /dev/md0 --chunk=64 --level=0 --raid-devices=$INSTANCE_STORE_COUNT $INSTANCE_STORE_LIST
mkfs.ext4 /dev/md0
mount /dev/md0 /mnt
chown ssm-user:ssm-user /mnt
