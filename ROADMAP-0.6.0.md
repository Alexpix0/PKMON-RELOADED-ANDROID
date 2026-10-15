# 🚀 ROADMAP & CAMBIOS v0.6.0 EXTREME PERFORMANCE

## 📌 Resumen de la Versión 0.6.0
* **Fecha de Lanzamiento:** 15 de Octubre de 2026
* **Versión:** `versionCode = 17`, `versionName = "0.6.0"`
* **Rama de respaldo:** `pre-inyecciones-dll` (guardada en GitHub `origin/pre-inyecciones-dll` para rollback inmediato si se requiere).

---

## ⚡ Los 4 Módulos de Rendimiento Inyectados

### 1. Microsoft `mimalloc` (High-Performance Memory Allocator)
* **Objetivo:** Erradicar la fragmentación de memoria y la contención de mutexes del asignador `malloc` por defecto de Android libc (Bionic).
* **Impacto:** Todas las estructuras dinámicas de Game Maker (arrays, strings, `Value`, instancias y vectores) en `libgm8emulator.so` ahora se asignan y liberan en piscinas de memoria por hilo ultrarrápidas.

### 2. Binary Shader Cache en Disco (`glProgramBinary`)
* **Objetivo:** Eliminar los micro-tirones y congelamientos al iniciar o cambiar de escena debido a la compilación de shaders GLSL en caliente.
* **Impacto:** Tras compilar la primera vez, el blob binario de GPU optimizado se guarda en `/data/data/com.klisee.reloaded/files/shader_cache/reloaded_main.bin`. Los arranques y cambios de sala posteriores cargan el shader precompilado en **0 ms**.

### 3. Multithreaded Background Asset Streamer
* **Objetivo:** Eliminar los tirones de disco flash de lectura aleatoria al entrar a combates de Pokémon salvajes o entrenadores.
* **Impacto:** Un hilo dedicado en segundo plano (`bg-asset-streamer`) utiliza los núcleos ociosos de la CPU (Cortex-A53) para descompilar y precalentar en RAM (`SPRITE_CACHE`) los sprites de combate y recursos de la interfaz mientras el usuario juega, evitando lecturas lentas de almacenamiento durante el combate.

### 4. HLE Native Fast-Math Plugins
* **Objetivo:** Máxima velocidad de cálculo para proyectiles, IA enemiga, vectores de movimiento y detección de rango.
* **Impacto:** Las funciones matemáticas críticas (`point_distance`, `point_distance_3d`, `point_direction`, `lengthdir_x`, `lengthdir_y`, `dot_product`, `dot_product_3d`, `move_towards_point`, `move_contact_solid`, `move_contact_all`, `move_outside_solid`, `move_outside_all`) operan directamente con instrucciones FPU intrínsecas de hardware (`hypot`, `atan2`, `sin_cos`, `rem_euclid`), ahorrando millones de ciclos de CPU por segundo.
