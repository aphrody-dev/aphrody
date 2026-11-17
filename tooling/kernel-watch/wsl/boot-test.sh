#!/bin/bash
# Direct kernel boot of a built WSL2 kernel in QEMU with a tiny busybox initramfs: shows the guest uname and whether the log holds a panic.
# usage: boot-test.sh BUILD_DIR [ACCEL]   ACCEL is kvm (default, with -cpu host) or tcg (with -cpu max, for a kernel tuned for a newer CPU than the host).
# It cannot exercise hv_sock, dxgkrnl (/dev/dxg), WSLg or any Hyper-V enlightenment; those exist only on a Windows host.
set -euo pipefail
build=$(readlink -f "$1"); accel=${2:-kvm}
work=$(mktemp -d); trap 'rm -rf "$work"' EXIT
bb=$(command -v busybox)
mkdir -p "$work/root"/{bin,proc,sys,dev,lib,lib64}
cp "$bb" "$work/root/bin/busybox"
for lib in $(ldd "$bb" | grep -oE '/[^ ]+'); do mkdir -p "$work/root$(dirname "$lib")"; cp "$lib" "$work/root$lib"; done
for tool in sh mount uname dmesg grep ls poweroff cat sleep; do ln -s busybox "$work/root/bin/$tool"; done
cat > "$work/root/init" <<'INIT'
#!/bin/sh
mount -t proc proc /proc; mount -t sysfs sys /sys; mount -t devtmpfs dev /dev
echo "WSLTEST uname $(uname -r)"
echo "WSLTEST bugs $(dmesg | grep -cE 'BUG:|Oops|Call Trace:|kernel panic|general protection')"
echo "WSLTEST dxg $(ls /dev/dxg 2>&1)"
echo "WSLTEST hv_vsock $(grep -c hv_sock /proc/net/protocols 2>/dev/null)"
echo "WSLTEST end"
poweroff -f
INIT
chmod +x "$work/root/init"
(cd "$work/root" && find . | cpio -o -H newc --quiet | gzip -9 > "$work/init.cpio.gz")
if [ "$accel" = kvm ]; then qemu_accel=(-enable-kvm -cpu host); else qemu_accel=(-accel tcg -cpu max); fi
started=$(date +%s)
timeout 600 qemu-system-x86_64 "${qemu_accel[@]}" -smp 2 -m 1024 -display none -monitor none -no-reboot \
  -kernel "$build/bzImage" -initrd "$work/init.cpio.gz" -append "console=ttyS0 panic=-1 loglevel=6" -serial file:"$build/boot-$accel.log" || true
echo "accel $accel seconds $(( $(date +%s) - started ))" > "$build/boot-$accel.meta"
grep -aE "Linux version|WSLTEST|Kernel panic|Oops" "$build/boot-$accel.log" | head -20
