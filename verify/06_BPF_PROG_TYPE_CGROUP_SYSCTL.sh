#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# Модуль 06: BPF_PROG_TYPE_CGROUP_SYSCTL
# Назначение: подсчёт обращений к sysctl в cgroup
# Хук: cgroup/sysctl — перехватывает чтение/запись sysctl
# Карта: sysctl_count (счётчик)
# Ожидание: при чтении sysctl-параметров счётчик увеличивается
# ═══════════════════════════════════════════════════════════════

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Модуль: BPF_PROG_TYPE_CGROUP_SYSCTL"
echo "[VERIFY] Функция: подсчёт обращений к sysctl в cgroup"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "[VERIFY] Действие 1: чтение net.ipv4.ip_forward"
VAL=$(sysctl -n net.ipv4.ip_forward 2>/dev/null)
echo "[VERIFY]   net.ipv4.ip_forward = $VAL"

echo ""
echo "[VERIFY] Действие 2: чтение net.ipv4.tcp_syncookies"
VAL=$(sysctl -n net.ipv4.tcp_syncookies 2>/dev/null)
echo "[VERIFY]   net.ipv4.tcp_syncookies = $VAL"

echo ""
echo "[VERIFY] Действие 3: чтение kernel.hostname"
VAL=$(sysctl -n kernel.hostname 2>/dev/null)
echo "[VERIFY]   kernel.hostname = $VAL"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Итог: 3 обращения к sysctl выполнены"
echo "[VERIFY] Проверьте в [RT] что sysctl_count увеличился на 3+"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
exit 0
