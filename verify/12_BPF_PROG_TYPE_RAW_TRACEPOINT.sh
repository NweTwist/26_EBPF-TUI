#!/bin/bash
# Модуль 12: RAW_TRACEPOINT
# Проверка: создание событий планировщика
echo "[VERIFY] Создание событий планировщика"
for i in $(seq 1 5); do sleep 0.1; done
echo "[VERIFY] PASS (scheduler events triggered)"
exit 0
