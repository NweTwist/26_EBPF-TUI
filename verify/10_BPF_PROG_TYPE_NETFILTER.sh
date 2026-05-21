#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# Модуль 10: BPF_PROG_TYPE_NETFILTER
# Назначение: подсчёт пакетов в netfilter (NF_ACCEPT)
# Хук: netfilter — перехватывает пакеты на уровне netfilter
# Карта: nf_count (счётчик пакетов)
# Ожидание: при сетевом трафике счётчик увеличивается
# ═══════════════════════════════════════════════════════════════

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Модуль: BPF_PROG_TYPE_NETFILTER"
echo "[VERIFY] Функция: подсчёт пакетов через netfilter hook"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "[VERIFY] Действие: отправка ICMP-пакетов через netfilter"
echo "[VERIFY] Команда: ping -c 3 127.0.0.1"
echo "[VERIFY] Ожидание: каждый пакет проходит netfilter, nf_count +1"
echo ""
ping -c 3 127.0.0.1 2>&1 | while read line; do echo "[VERIFY]   $line"; done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Итог: 6+ пакетов через netfilter (request+reply)"
echo "[VERIFY] Проверьте в [RT] что nf_count увеличился"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
exit 0
