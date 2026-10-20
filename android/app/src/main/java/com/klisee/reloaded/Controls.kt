package com.klisee.reloaded

import android.content.Context
import android.hardware.input.InputManager
import android.view.InputDevice
import android.view.KeyEvent
import java.io.File

/** The pads Android is reporting right now. */
fun connectedPads(context: Context): List<String> {
    val manager = context.getSystemService(Context.INPUT_SERVICE) as InputManager
    val names = mutableListOf<String>()
    for (id in manager.inputDeviceIds) {
        val device = manager.getInputDevice(id) ?: continue
        val gamepad = device.sources and InputDevice.SOURCE_GAMEPAD == InputDevice.SOURCE_GAMEPAD
        val joystick = device.sources and InputDevice.SOURCE_JOYSTICK == InputDevice.SOURCE_JOYSTICK
        if (!device.isVirtual && (gamepad || joystick)) names.add(device.name)
    }
    return names
}

/**
 * The keyboard keys the game reads that a pad button or PC key can be bound to.
 */
enum class ControlAction(val key: String, val defaultCode: Int, val label: String) {
    Z("Z", 96, "Acción Principal (Z)"),
    X("X", 97, "Cancelar / Correr (X)"),
    C("C", 99, "Secundario / Bici (C)"),
    V("V", 100, "Caña / Ataque 4 (V)"),
    ENTER("RETURN", 109, "Menú / OK (Enter)"),
    SPACE("SPACE", 108, "Equipo Pokémon (Espacio)"),
    B("B", 103, "Relevo Pokémon (B)"),
    M("M", 102, "Objeto Rápido / Poción (M)"),
    O("O", 105, "Mochila (O)"),
    S("S", 104, "Megaevolución (S)"),
    E("E", 106, "Huir / Escapar (E)"),
    UP("UP", 19, "Mover Arriba (↑)"),
    DOWN("DOWN", 20, "Mover Abajo (↓)"),
    LEFT("LEFT", 21, "Mover Izquierda (←)"),
    RIGHT("RIGHT", 22, "Mover Derecha (→)");
}

/** What the key does in the game, for the ones that are known. */
val ControlAction.hint: String
    get() = when (this) {
        ControlAction.Z -> "Interactuar, confirmar diálogos y ataque principal"
        ControlAction.X -> "Cancelar, correr en mundo y ataque secundario"
        ControlAction.C -> "Bicicleta, abrir menú y tercer ataque"
        ControlAction.V -> "Caña de pescar y cuarto ataque"
        ControlAction.ENTER -> "Menú de pausa y confirmación"
        ControlAction.SPACE -> "Abrir pantalla del equipo Pokémon"
        ControlAction.B -> "Relevo de Pokémon en combate"
        ControlAction.M -> "Usar objeto o poción rápida en combate"
        ControlAction.O -> "Abrir inventario y mochila"
        ControlAction.S -> "Megaevolución, Movimiento Z o Dinamax"
        ControlAction.E -> "Escapar de Pokémon salvaje o huir de batalla"
        ControlAction.UP -> "Caminar o navegar hacia arriba"
        ControlAction.DOWN -> "Caminar o navegar hacia abajo"
        ControlAction.LEFT -> "Caminar o navegar hacia la izquierda"
        ControlAction.RIGHT -> "Caminar o navegar hacia la derecha"
    }

/** The name of a pad button or PC keyboard key, for showing it on screen. */
fun buttonName(code: Int): String = when (code) {
    // Gamepad buttons
    96 -> "Mando A"
    97 -> "Mando B"
    99 -> "Mando X"
    100 -> "Mando Y"
    102 -> "Mando L1"
    103 -> "Mando R1"
    104 -> "Mando L2"
    105 -> "Mando R2"
    106 -> "Mando Stick L3"
    107 -> "Mando Stick R3"
    108 -> "Mando Start"
    109 -> "Mando Select"

    // D-Pad
    KeyEvent.KEYCODE_DPAD_UP -> "D-Pad Arriba"
    KeyEvent.KEYCODE_DPAD_DOWN -> "D-Pad Abajo"
    KeyEvent.KEYCODE_DPAD_LEFT -> "D-Pad Izquierda"
    KeyEvent.KEYCODE_DPAD_RIGHT -> "D-Pad Derecha"

    // PC Keyboard Special Keys
    KeyEvent.KEYCODE_ENTER -> "Tecla ENTER"
    KeyEvent.KEYCODE_SPACE -> "Tecla ESPACIO"
    KeyEvent.KEYCODE_ESCAPE -> "Tecla ESCAPE"
    KeyEvent.KEYCODE_TAB -> "Tecla TAB"
    KeyEvent.KEYCODE_DEL -> "Tecla RETROCESO"
    KeyEvent.KEYCODE_SHIFT_LEFT -> "Shift Izquierdo"
    KeyEvent.KEYCODE_SHIFT_RIGHT -> "Shift Derecho"
    KeyEvent.KEYCODE_CTRL_LEFT -> "Ctrl Izquierdo"
    KeyEvent.KEYCODE_CTRL_RIGHT -> "Ctrl Derecho"
    KeyEvent.KEYCODE_ALT_LEFT -> "Alt Izquierdo"
    KeyEvent.KEYCODE_ALT_RIGHT -> "Alt Derecho"

    // PC Numbers (0..9)
    in KeyEvent.KEYCODE_0..KeyEvent.KEYCODE_9 -> "Tecla ${code - KeyEvent.KEYCODE_0}"

    // PC Letters (A..Z)
    in KeyEvent.KEYCODE_A..KeyEvent.KEYCODE_Z -> "Tecla ${('A'.code + (code - KeyEvent.KEYCODE_A)).toChar()}"

    // PC Function Keys (F1..F12)
    in KeyEvent.KEYCODE_F1..KeyEvent.KEYCODE_F12 -> "F${code - KeyEvent.KEYCODE_F1 + 1}"

    else -> {
        val raw = KeyEvent.keyCodeToString(code)
        if (raw.startsWith("KEYCODE_")) {
            raw.removePrefix("KEYCODE_")
        } else {
            "Botón $code"
        }
    }
}

/** Definition for key item choice in manual picker */
data class PcKeyChoice(val code: Int, val name: String, val category: String)

/** Full PC Keyboard & Gamepad Keys Catalogue for visual custom assignment */
val AVAILABLE_KEY_CHOICES: List<PcKeyChoice> by lazy {
    val list = mutableListOf<PcKeyChoice>()

    // Gamepad
    list.add(PcKeyChoice(96, "Mando A", "Mando"))
    list.add(PcKeyChoice(97, "Mando B", "Mando"))
    list.add(PcKeyChoice(99, "Mando X", "Mando"))
    list.add(PcKeyChoice(100, "Mando Y", "Mando"))
    list.add(PcKeyChoice(102, "Mando L1 (LB)", "Mando"))
    list.add(PcKeyChoice(103, "Mando R1 (RB)", "Mando"))
    list.add(PcKeyChoice(104, "Mando L2 (LT)", "Mando"))
    list.add(PcKeyChoice(105, "Mando R2 (RT)", "Mando"))
    list.add(PcKeyChoice(106, "Mando Stick L3", "Mando"))
    list.add(PcKeyChoice(107, "Mando Stick R3", "Mando"))
    list.add(PcKeyChoice(108, "Mando Start", "Mando"))
    list.add(PcKeyChoice(109, "Mando Select", "Mando"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_UP, "D-Pad Arriba", "Mando"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_DOWN, "D-Pad Abajo", "Mando"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_LEFT, "D-Pad Izquierda", "Mando"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_RIGHT, "D-Pad Derecha", "Mando"))

    // PC Letters A-Z
    for (c in 'A'..'Z') {
        val code = KeyEvent.KEYCODE_A + (c.code - 'A'.code)
        list.add(PcKeyChoice(code, "Tecla $c", "Letras"))
    }

    // PC Numbers 0-9
    for (d in 0..9) {
        val code = KeyEvent.KEYCODE_0 + d
        list.add(PcKeyChoice(code, "Tecla $d", "Números"))
    }

    // Special Keys
    list.add(PcKeyChoice(KeyEvent.KEYCODE_SPACE, "Espacio", "Especiales"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_ENTER, "Enter / Return", "Especiales"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_ESCAPE, "Escape (Esc)", "Especiales"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_TAB, "Tabulador", "Especiales"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DEL, "Retroceso / Backspace", "Especiales"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_SHIFT_LEFT, "Shift Izquierdo", "Especiales"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_SHIFT_RIGHT, "Shift Derecho", "Especiales"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_CTRL_LEFT, "Control Izquierdo", "Especiales"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_CTRL_RIGHT, "Control Derecho", "Especiales"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_ALT_LEFT, "Alt Izquierdo", "Especiales"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_ALT_RIGHT, "Alt Derecho", "Especiales"))

    // Arrow keys
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_UP, "Flecha Arriba (↑)", "Flechas"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_DOWN, "Flecha Abajo (↓)", "Flechas"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_LEFT, "Flecha Izquierda (←)", "Flechas"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_RIGHT, "Flecha Derecha (→)", "Flechas"))

    // Function keys F1-F12
    for (i in 1..12) {
        val code = KeyEvent.KEYCODE_F1 + (i - 1)
        list.add(PcKeyChoice(code, "F$i", "Funciones (F1-F12)"))
    }

    list
}

/**
 * Writes the layout the emulator reads when it starts.
 */
fun writeControlsFile(
    filesDir: File,
    bindings: Map<ControlAction, Int>,
    stick: Boolean,
    padConnected: Boolean,
    opacity: Int = 70,
    scale: Int = 100,
    marginX: Int = 0,
    marginY: Int = 0,
    haptics: Boolean = true,
    deadzone: Int = 35,
    showNumKeys: Boolean = true,
) {
    val text = buildString {
        appendLine("# Escrito por la pantalla de Controles. El emulador lo lee al arrancar.")
        for (action in ControlAction.entries) {
            val code = bindings[action] ?: action.defaultCode
            appendLine("$code=${action.key}")
        }
        appendLine("stick=${if (stick) 1 else 0}")
        appendLine("pad=${if (padConnected) 1 else 0}")
        appendLine("opacity=$opacity")
        appendLine("scale=$scale")
        appendLine("margin_x=$marginX")
        appendLine("margin_y=$marginY")
        appendLine("haptics=${if (haptics) 1 else 0}")
        appendLine("deadzone=$deadzone")
        appendLine("show_num_keys=${if (showNumKeys) 1 else 0}")
    }
    runCatching { File(filesDir, "controls.conf").writeText(text) }
}
