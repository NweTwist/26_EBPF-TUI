#!/bin/bash
# Модуль 02: CGROUP_SKB
# Проверка: создание egress-трафика из текущего cgroup
echo "[VERIFY] Создание сетевого трафика (ping loopback)"
ping -c 3 127.0.0.1 > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "[VERIFY] PASS (egress traffic generated)"
    exit 0
else
    echo "[VERIFY] FAIL (ping failed)"
    exit 1
fi
