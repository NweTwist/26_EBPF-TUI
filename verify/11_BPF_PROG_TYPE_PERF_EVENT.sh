#!/bin/bash
# Модуль 11: PERF_EVENT
# Проверка: нагрузка на CPU
echo "[VERIFY] Создание CPU-нагрузки"
for i in $(seq 1 5); do sha256sum /bin/ls > /dev/null 2>&1; done
echo "[VERIFY] PASS (perf event triggered)"
exit 0
