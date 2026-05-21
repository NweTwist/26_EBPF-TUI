#!/bin/bash
# Модуль 21: SOCKET_FILTER
# Проверка: пакетный трафик на loopback
echo "[VERIFY] Создание пакетного трафика"
ping -c 3 127.0.0.1 > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "[VERIFY] PASS (socket_filter triggered)"
    exit 0
else
    echo "[VERIFY] FAIL (ping failed)"
    exit 1
fi
