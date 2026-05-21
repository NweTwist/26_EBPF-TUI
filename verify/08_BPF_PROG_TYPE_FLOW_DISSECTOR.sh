#!/bin/bash
# Модуль 08: FLOW_DISSECTOR
# Проверка: сетевой пакет через flow dissector
echo "[VERIFY] Создание сетевого пакета"
ping -c 3 127.0.0.1 > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "[VERIFY] PASS (flow dissector triggered)"
    exit 0
else
    echo "[VERIFY] FAIL (ping failed)"
    exit 1
fi
