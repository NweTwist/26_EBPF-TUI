#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# Модуль 14: BPF_PROG_TYPE_SCHED_ACT
# Назначение: подсчёт пакетов в TC action (traffic control)
# Хук: action — TC action, обрабатывает пакеты в qdisc
# Карта: act_count (счётчик пакетов)
# Ожидание: при трафике через TC act_count увеличивается
# ═══════════════════════════════════════════════════════════════

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Модуль: BPF_PROG_TYPE_SCHED_ACT"
echo "[VERIFY] Функция: TC action — подсчёт пакетов в qdisc"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "[VERIFY] Действие: генерация egress-трафика через loopback"
echo "[VERIFY] Команда: ping -c 3 127.0.0.1"
echo "[VERIFY] Ожидание: пакеты проходят через TC action, act_count +N"
echo ""
ping -c 3 127.0.0.1 2>&1 | while read line; do echo "[VERIFY]   $line"; done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Итог: пакеты отправлены через TC qdisc"
echo "[VERIFY] Проверьте в [RT] что act_count увеличился"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
exit 0
