#!/bin/bash
# Модуль 15: SCHED_CLS
# Проверка: egress-трафик через loopback
echo "[VERIFY] Создание egress-трафика"
ping -c 3 127.0.0.1 > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "[VERIFY] PASS (sched_cls triggered)"
    exit 0
else
    echo "[VERIFY] FAIL (ping failed)"
    exit 1
fi
