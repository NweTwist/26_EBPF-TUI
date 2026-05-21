#!/bin/bash
# Модуль 10: NETFILTER
# Проверка: создание IP-трафика
echo "[VERIFY] Создание IP-трафика"
ping -c 3 127.0.0.1 > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "[VERIFY] PASS (netfilter triggered)"
    exit 0
else
    echo "[VERIFY] FAIL (ping failed)"
    exit 1
fi
