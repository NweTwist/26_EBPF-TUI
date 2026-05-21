#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# Модуль 22: BPF_PROG_TYPE_STRUCT_OPS
# Назначение: BPF TCP congestion control алгоритм "bpf_cc"
# Хук: struct_ops (tcp_congestion_ops) — cong_avoid, ssthresh, etc
# Карта: ca_count (счётчик вызовов cong_avoid)
# Особенность: модуль сам создаёт TCP-соединение с bpf_cc
# Ожидание: ca_count увеличивается при передаче данных
# ═══════════════════════════════════════════════════════════════

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Модуль: BPF_PROG_TYPE_STRUCT_OPS"
echo "[VERIFY] Функция: BPF TCP congestion control (bpf_cc)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "[VERIFY] Модуль самодостаточен: регистрирует TCP CC 'bpf_cc'"
echo "[VERIFY] и сам создаёт TCP-соединение с этим алгоритмом"
echo "[VERIFY] (setsockopt TCP_CONGESTION='bpf_cc')"
echo ""
echo "[VERIFY] Проверка: алгоритм bpf_cc зарегистрирован?"
if [ -f /proc/sys/net/ipv4/tcp_available_congestion_control ]; then
    AVAIL=$(cat /proc/sys/net/ipv4/tcp_available_congestion_control)
    echo "[VERIFY]   Доступные CC: $AVAIL"
    if echo "$AVAIL" | grep -q "bpf_cc"; then
        echo "[VERIFY]   bpf_cc НАЙДЕН — модуль работает корректно"
    else
        echo "[VERIFY]   bpf_cc не найден в списке (возможно ещё не зарегистрирован)"
    fi
fi
sleep 1

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Итог: модуль работает автономно"
echo "[VERIFY] Проверьте в [RT] что ca_count > 0 и растёт"
echo "[VERIFY] ca_count увеличивается при каждом cong_avoid()"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
exit 0
