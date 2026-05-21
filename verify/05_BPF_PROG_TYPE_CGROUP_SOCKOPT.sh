#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# Модуль 05: BPF_PROG_TYPE_CGROUP_SOCKOPT
# Назначение: подсчёт вызовов setsockopt() в cgroup
# Хук: cgroup/setsockopt — перехватывает каждый setsockopt()
# Карта: setsockopt_count (счётчик)
# Ожидание: при вызове setsockopt счётчик увеличивается
# ═══════════════════════════════════════════════════════════════

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Модуль: BPF_PROG_TYPE_CGROUP_SOCKOPT"
echo "[VERIFY] Функция: подсчёт setsockopt() вызовов в cgroup"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "[VERIFY] Действие: вызов setsockopt(SO_REUSEADDR) 3 раза"
echo "[VERIFY] Ожидание: setsockopt_count +3"
python3 -c "
import socket
for i in range(3):
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    print(f'[VERIFY]   setsockopt #{i+1}: SO_REUSEADDR=1 — выполнено')
    s.close()
print('[VERIFY]   Все 3 вызова setsockopt завершены')
"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Итог: 3 вызова setsockopt()"
echo "[VERIFY] Проверьте в [RT] что setsockopt_count увеличился на 3+"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
exit 0
