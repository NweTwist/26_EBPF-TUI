#!/bin/bash
# Модуль 06: CGROUP_SYSCTL
# Проверка: обращение к sysctl
echo "[VERIFY] Обращение к sysctl"
sysctl net.ipv4.ip_forward > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "[VERIFY] PASS (sysctl access triggered)"
    exit 0
else
    echo "[VERIFY] FAIL (sysctl failed)"
    exit 1
fi
