#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# Модуль 11: BPF_PROG_TYPE_PERF_EVENT
# Назначение: подсчёт событий perf (CPU cycles, instructions)
# Хук: perf_event — срабатывает при overflow perf-счётчика
# Карта: perf_count (счётчик событий)
# Ожидание: при CPU-нагрузке счётчик увеличивается
# ═══════════════════════════════════════════════════════════════

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Модуль: BPF_PROG_TYPE_PERF_EVENT"
echo "[VERIFY] Функция: подсчёт hardware perf events (CPU)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "[VERIFY] Действие: создание CPU-нагрузки (sha256sum x5)"
echo "[VERIFY] Команда: sha256sum /bin/ls (5 раз)"
echo "[VERIFY] Ожидание: perf overflow events → perf_count +N"
echo ""
for i in $(seq 1 5); do
    sha256sum /bin/ls > /dev/null 2>&1
    echo "[VERIFY]   sha256sum #$i выполнен"
done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Итог: 5 вычислений SHA-256 создали CPU-нагрузку"
echo "[VERIFY] Проверьте в [RT] что perf_count увеличился"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
exit 0
