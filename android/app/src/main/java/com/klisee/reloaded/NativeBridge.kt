package com.klisee.reloaded

/**
 * Puente JNI entre la interfaz de Android y el motor nativo Rust.
 */
object NativeBridge {
    init {
        try {
            System.loadLibrary("gm8emulator")
        } catch (_: UnsatisfiedLinkError) {
        }
    }

    external fun setVsyncEnabled(enabled: Boolean)
    external fun setAdaptiveFrameskip(enabled: Boolean)
}
