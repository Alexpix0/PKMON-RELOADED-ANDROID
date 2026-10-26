0.3.6 Latest
@Klisee Klisee released this 6 hours ago
 v0.3.6
 4cecd6c
Versión 0.3.6

Estabilidad

Al volver del segundo plano: ya no se queda colgado al reabrir el juego después de un rato minimizado. Arreglado todo lo que se quedaba a medias: la ventana nativa, la superficie EGL, el audio y el estado de entrada.
Contexto gráfico: si el driver se deja por el camino el contexto de OpenGL, el juego cierra la sesión limpiamente en lugar de quedarse plantado.
Audio: el juego ahora pide el foco de audio y lo suelta al pausarse, así que ya no se cierra de golpe al reanudar en Android 14.
Entrada al juego: la ventana arranca a pantalla completa y en negro, sin ese destello que aparecía antes.
Compatibilidad

Android 8 en adelante: bajamos el mínimo desde Android 10, así que ahora entra mucha más gente.
Un APK por arquitectura: arm64-v8a, armeabi-v7a, x86_64 y universal.
Actualizaciones

El actualizador ya no va a ciegas: elige el APK según la arquitectura del móvil, comprueba el SHA-256 y verifica firma, versión y ABI antes de instalar nada.
Créditos

@nyxthor-dev: por el análisis del congelamiento y las propuestas de corrección en el issue #77.