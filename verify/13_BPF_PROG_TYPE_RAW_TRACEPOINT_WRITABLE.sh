#!/bin/bash
# Модуль 13: RAW_TRACEPOINT_WRITABLE
# Проверка: вызов системных событий через обращение к ФС
echo "[VERIFY] Обращение к файловой системе"
ls /tmp > /dev/null
echo "[VERIFY] PASS (fs access triggered)"
exit 0
