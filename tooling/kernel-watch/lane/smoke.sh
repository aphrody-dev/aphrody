#!/bin/bash
# Runs inside the guest as a systemd service after boot and reports on the serial console with LANE-RESULT and LANE-METRIC lines.
set -u
export TERM=dumb NO_COLOR=1 SYSTEMD_COLORS=0 SYSTEMD_URLS=0 SYSTEMD_PAGER=
. /etc/lane.env
KPKG_NAME=""

# Lines go to the console and to a second serial port (ttyS1) that nothing else writes to, so getty and systemd output cannot split them.
emit() { echo "$*"; echo "$*" > /dev/ttyS1 2>/dev/null; }
result() { emit "LANE-RESULT $1 $2 $3"; }
metric() { emit "LANE-METRIC $1 $2"; }
finish() { emit "LANE-END"; sync; systemctl --no-block poweroff; }
trap finish EXIT

release=$(uname -r)
emit "LANE-BEGIN $release"
KPKG_NAME=$(pacman -Qqo "/usr/lib/modules/$release/vmlinuz" 2>/dev/null | head -1)

state=$(systemctl is-system-running --wait 2>&1 || true)
failed=$(systemctl --failed --no-legend --plain 2>/dev/null | awk '{print $1}' | tr '\n' ',')
case "$state" in
  running) result boot pass "systemd $state" ;;
  degraded) result boot pass "systemd degraded, failed units: ${failed:-none}" ;;
  *) result boot fail "systemd $state, failed units: ${failed:-none}" ;;
esac
getty=$(systemctl is-active serial-getty@ttyS0.service 2>&1)
if [ "$getty" = active ]; then result login pass "serial-getty@ttyS0 $getty"; else result login fail "serial-getty@ttyS0 $getty"; fi

count() { dmesg --level="$1" 2>/dev/null | wc -l; }
err=$(count err) warn=$(count warn) crit=$(count crit,alert,emerg)
bug=$(dmesg 2>/dev/null | grep -cE 'BUG:|Oops|Call Trace:|general protection fault|kernel panic')
metric dmesg-err "$err"; metric dmesg-warn "$warn"; metric dmesg-crit "$crit"; metric dmesg-bug "$bug"
dmesg --level=err,crit,alert,emerg 2>/dev/null | head -20 | sed 's/^/LANE-DMESG /' | while read -r l; do emit "$l"; done
if [ "$bug" = 0 ] && [ "$crit" = 0 ]; then result dmesg pass "err=$err warn=$warn crit=$crit bug=$bug"; else result dmesg fail "err=$err warn=$warn crit=$crit bug=$bug"; fi

analyze=$(systemd-analyze time 2>&1 | head -1)
secs=$(systemd-analyze time 2>/dev/null | head -1 | grep -oE '= [0-9.]+(min [0-9.]+)?s' | head -1)
if [ -n "$analyze" ]; then result analyze pass "$analyze"; else result analyze fail "no output"; fi
systemd-analyze blame 2>/dev/null | head -5 | sed 's/^/LANE-BLAME /' | while read -r l; do emit "$l"; done

net=fail
for _ in $(seq 1 45); do
  if getent hosts archlinux.org >/dev/null 2>&1; then net=pass; break; fi
  sleep 1
done
result network "$net" "dns archlinux.org through the user-mode network"

if [ "$net" = pass ]; then
  for repo in $TESTING_REPOS; do
    base=${repo%-testing}
    grep -q "^\[$repo\]" /etc/pacman.conf || sed -i "/^\[$base\]/i [$repo]\nInclude = /etc/pacman.d/mirrorlist\n" /etc/pacman.conf
  done
  t0=$(date +%s)
  ignore=""; [ -n "$KPKG_NAME" ] && ignore="--ignore $KPKG_NAME"
  timeout 900 pacman -Syu --noconfirm --noprogressbar $ignore > /tmp/syu.log 2>&1
  rc=$?
  n=$(grep -E '^Packages \(' /tmp/syu.log | head -1 | grep -oE '\([0-9]+\)' | tr -d '()')
  metric pacman-syu-seconds $(( $(date +%s) - t0 ))
  if [ "$rc" = 0 ]; then result pacman-syu pass "exit 0 with ${TESTING_REPOS} enabled, ${n:-0} packages upgraded, kernel package ${KPKG_NAME:-unknown} held"; else result pacman-syu fail "exit $rc"; fi
  for repo in $TESTING_REPOS; do pacman -Sl "$repo" 2>/dev/null | awk '$2 ~ /^linux(-[a-z]+)?$/ { print "LANE-TESTING " $2 " " $1 " " $3 }'; done | while read -r l; do emit "$l"; done
else
  result pacman-syu skip "no network"
fi

pacman -Qkk > /tmp/qkk.out 2> /tmp/qkk.err
# Owner and group of directories that systemd creates at runtime (/var/log/journal and the like) legitimately differ from the package manifest.
grep -E '(warning|error):' /tmp/qkk.err | grep -vE ': /(var|run)/[^ ]* \((UID|GID) mismatch\)' > /tmp/qkk.real
runtime=$(grep -cE ': /(var|run)/[^ ]* \((UID|GID) mismatch\)' /tmp/qkk.err)
bad=$(wc -l < /tmp/qkk.real)
head -5 /tmp/qkk.real | sed 's/^/LANE-QKK /' | while read -r l; do emit "$l"; done
if [ "$bad" = 0 ]; then result pacman-qkk pass "$(grep -c . /tmp/qkk.out) packages checked, 0 diagnostics, $runtime runtime-state ownership notes ignored"; else result pacman-qkk fail "$bad diagnostics, $runtime runtime-state ownership notes ignored"; fi

if [ -x /opt/kselftest/run_kselftest.sh ]; then
  ( cd /opt/kselftest && timeout 300 ./run_kselftest.sh > /tmp/kselftest.log 2>&1 )
  krc=$?
  pass=$(grep -cE '^ok ' /tmp/kselftest.log); notok=$(grep -cE '^not ok ' /tmp/kselftest.log)
  metric kselftest-ok "$pass"; metric kselftest-notok "$notok"
  grep -E '^not ok ' /tmp/kselftest.log | sed 's/^not ok [0-9]* //' | head -8 | sed 's/^/LANE-KSELFTEST /' | while read -r l; do emit "$l"; done
  # Whether the failures are regressions is decided on the host against the baseline kernel; here only "did the suite run" is judged.
  if [ "$pass" -gt 0 ]; then result kselftest pass "$pass ok, $notok not ok (exit $krc)"; else result kselftest fail "no test completed (exit $krc)"; fi
else
  result kselftest skip "no kselftest build"
fi
