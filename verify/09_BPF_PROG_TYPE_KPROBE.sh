#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# Модуль 09: BPF_PROG_TYPE_KPROBE
# Назначение: перехват вызовов ядерной функции do_sys_openat2
# Хук: kprobe/do_sys_openat2 — срабатывает при каждом open/openat
# Карта: kprobe_count (счётчик вызовов openat)
# Ожидание: при открытии файлов счётчик увеличивается
# ═══════════════════════════════════════════════════════════════

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Модуль: BPF_PROG_TYPE_KPROBE"
echo "[VERIFY] Функция: перехват openat() через kprobe"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "[VERIFY] Действие 1: создание файла /tmp/kprobe_verify_test"
echo "[VERIFY] Команда: touch /tmp/kprobe_verify_test"
echo "[VERIFY] Ожидание: openat() → kprobe_count +1"
touch /tmp/kprobe_verify_test
echo "[VERIFY]   Файл создан"

echo ""
echo "[VERIFY] Действие 2: чтение файла"
echo "[VERIFY] Команда: cat /tmp/kprobe_verify_test"
echo "[VERIFY] Ожидание: openat() → kprobe_count +1"
cat /tmp/kprobe_verify_test > /dev/null 2>&1
echo "[VERIFY]   Файл прочитан"

echo ""
echo "[VERIFY] Действие 3: листинг каталога /tmp"
echo "[VERIFY] Команда: ls /tmp"
echo "[VERIFY] Ожидание: множество openat() для чтения директории"
ls /tmp > /dev/null 2>&1
echo "[VERIFY]   Каталог прочитан"

rm -f /tmp/kprobe_verify_test

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "[VERIFY] Итог: 3+ вызова openat() выполнены"
echo "[VERIFY] Проверьте в [RT] что kprobe_count увеличился"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
exit 0
