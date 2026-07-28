#!/usr/bin/env bash
set -euo pipefail

action="${1:-status}"
serial="${2:-}"
package="fail.acheron.floatview"

if [[ -z "$serial" ]]; then
  serial="$(adb devices | awk 'NR > 1 && $2 == "device" { print $1 }')"
  count="$(printf '%s\n' "$serial" | sed '/^$/d' | wc -l | tr -d ' ')"
  if [[ "$count" != "1" ]]; then
    echo "Expected exactly one authorized Android device; pass its serial explicitly:" >&2
    adb devices -l >&2
    exit 1
  fi
fi

sdk="$(adb -s "$serial" shell getprop ro.build.version.sdk | tr -d '\r')"
if [[ ! "$sdk" =~ ^[0-9]+$ ]]; then
  echo "Could not determine Android API level for $serial." >&2
  exit 1
fi

if (( sdk >= 31 )); then
  permissions=(
    android.permission.BLUETOOTH_SCAN
    android.permission.BLUETOOTH_CONNECT
  )
else
  permissions=(android.permission.ACCESS_FINE_LOCATION)
fi
revoke_permissions=("${permissions[@]}")
if (( sdk < 31 )); then
  revoke_permissions=(
    android.permission.ACCESS_FINE_LOCATION
    android.permission.ACCESS_COARSE_LOCATION
  )
fi

case "$action" in
  status)
    echo "Android API $sdk Bluetooth runtime permission state:"
    adb -s "$serial" shell dumpsys package "$package" |
      grep -E 'ACCESS_FINE_LOCATION|BLUETOOTH_SCAN|BLUETOOTH_CONNECT' || true
    ;;
  deny)
    for permission in "${revoke_permissions[@]}"; do
      adb -s "$serial" shell pm revoke "$package" "$permission"
    done
    adb -s "$serial" shell am force-stop "$package"
    echo "Denied Bluetooth runtime permissions and stopped Float View."
    ;;
  grant)
    for permission in "${permissions[@]}"; do
      adb -s "$serial" shell pm grant "$package" "$permission"
    done
    echo "Granted Bluetooth runtime permissions."
    ;;
  reset)
    adb -s "$serial" shell am force-stop "$package"
    history="shared_prefs/bluetooth-permission-history.xml"
    backup="files/bluetooth-permission-history.reset.xml"
    if adb -s "$serial" shell run-as "$package" ls "$history" >/dev/null 2>&1; then
      adb -s "$serial" shell run-as "$package" mv "$history" "$backup"
    fi
    for permission in "${permissions[@]}"; do
      adb -s "$serial" shell pm grant "$package" "$permission"
    done
    for permission in "${revoke_permissions[@]}"; do
      adb -s "$serial" shell pm revoke "$package" "$permission"
      if (( sdk >= 30 )); then
        adb -s "$serial" shell pm clear-permission-flags \
          "$package" "$permission" user-set user-fixed
      fi
    done
    echo "Reset Float View to the pre-prompt Bluetooth permission state."
    ;;
  *)
    echo "Usage: android-r6-permission.sh <status|deny|grant|reset> [serial]" >&2
    exit 2
    ;;
esac
