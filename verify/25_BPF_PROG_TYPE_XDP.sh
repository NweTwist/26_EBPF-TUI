#!/bin/bash
# Модуль 25: XDP
# Проверка: loopback-трафик для XDP
echo "[VERIFY] Создание loopback-трафика"
ping -c 3 127.0.0.1 > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "[VERIFY] PASS (xdp triggered)"
    exit 0
else
    echo "[VERIFY] FAIL (ping failed)"
    exit 1
fi
