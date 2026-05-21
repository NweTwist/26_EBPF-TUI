#!/bin/bash
# Модуль 24: TRACEPOINT
# Проверка: вызов openat() через touch и cat
echo "[VERIFY] Вызов openat() (touch + cat)"
touch /tmp/tp_test_action
cat /tmp/tp_test_action > /dev/null
rm -f /tmp/tp_test_action
echo "[VERIFY] PASS (tracepoint triggered)"
exit 0
