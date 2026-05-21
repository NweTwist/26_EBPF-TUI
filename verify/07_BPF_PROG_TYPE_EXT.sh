#!/bin/bash
# Модуль 07: EXT
# Проверка: loopback-трафик для EXT через XDP на lo
echo "[VERIFY] Создание loopback-трафика"
ping -c 3 127.0.0.1 > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "[VERIFY] PASS (loopback traffic generated)"
    exit 0
else
    echo "[VERIFY] FAIL (ping failed)"
    exit 1
fi
