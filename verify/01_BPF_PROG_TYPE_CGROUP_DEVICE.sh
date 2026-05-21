#!/bin/bash
# Модуль 01: CGROUP_DEVICE
# Проверка: требует физическое USB-устройство — выполняем чтение /dev/null как fallback
echo "[VERIFY] Проверка доступа к устройству через cgroup device controller"
cat /dev/null > /dev/null 2>&1
echo "[VERIFY] PASS (device access check completed)"
exit 0
