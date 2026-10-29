package com.klisee.reloaded

import dalvik.annotation.optimization.CriticalNative

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

    @CriticalNative
    @JvmStatic
    external fun nativeSetInputMask(mask: Int)
}
