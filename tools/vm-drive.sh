#!/bin/bash
# Drive the Windows VM (libvirt domain "win10") through QMP input events and
# screenshots, used to click through the original software while capturing USB.
# Needs membership of the libvirt group only.
# usage: vm.sh click X Y | vm.sh move X Y | vm.sh rclick X Y | vm.sh shot NAME | vm.sh key KEYNAME
V="virsh -c qemu:///system"
ax(){ python3 -c "print(round($1*32767/1920))"; }
ay(){ python3 -c "print(round($1*32767/1080))"; }
case "$1" in
 move) $V qemu-monitor-command win10 "{\"execute\":\"input-send-event\",\"arguments\":{\"events\":[{\"type\":\"abs\",\"data\":{\"axis\":\"x\",\"value\":$(ax $2)}},{\"type\":\"abs\",\"data\":{\"axis\":\"y\",\"value\":$(ay $3)}}]}}" >/dev/null;;
 click|rclick|dclick)
   b=left; [ "$1" = rclick ] && b=right
   $0 move $2 $3; sleep 0.15
   n=1; [ "$1" = dclick ] && n=2
   for i in $(seq $n); do
   $V qemu-monitor-command win10 "{\"execute\":\"input-send-event\",\"arguments\":{\"events\":[{\"type\":\"btn\",\"data\":{\"down\":true,\"button\":\"$b\"}}]}}" >/dev/null; sleep 0.08
   $V qemu-monitor-command win10 "{\"execute\":\"input-send-event\",\"arguments\":{\"events\":[{\"type\":\"btn\",\"data\":{\"down\":false,\"button\":\"$b\"}}]}}" >/dev/null; sleep 0.08
   done;;
 shot) sleep ${3:-0.8}; $V screenshot win10 "$2.png" >/dev/null; echo "$2.png";;
 key) $V send-key win10 $2 >/dev/null;;
esac
