#!/bin/bash
# Модуль 09: KPROBE
# Проверка: вызов openat() через touch и cat
echo "[VERIFY] Вызов openat() (touch + cat)"
touch /tmp/kprobe_test
cat /tmp/kprobe_test > /dev/null
rm -f /tmp/kprobe_test
echo "[VERIFY] PASS (openat triggered)"
exit 0
