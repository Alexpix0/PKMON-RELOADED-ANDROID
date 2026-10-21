package com.klisee.reloaded

import android.content.Context
import android.hardware.input.InputManager
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
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
data class PcKeyChoice(
    val code: Int,
    val name: String,
    val category: String,
    val keyTag: String = name
)

/** Full PC Keyboard & Gamepad Keys Catalogue for visual custom assignment */
val AVAILABLE_KEY_CHOICES: List<PcKeyChoice> by lazy {
    val list = mutableListOf<PcKeyChoice>()

    // Letras A-Z
    for (c in 'A'..'Z') {
        val code = KeyEvent.KEYCODE_A + (c.code - 'A'.code)
        list.add(PcKeyChoice(code, "Tecla $c", "Letras", "$c"))
    }

    // Números 0-9
    for (d in 0..9) {
        val code = KeyEvent.KEYCODE_0 + d
        list.add(PcKeyChoice(code, "Número $d", "Números", "$d"))
    }
    // Numpad 0-9
    for (d in 0..9) {
        val code = KeyEvent.KEYCODE_NUMPAD_0 + d
        list.add(PcKeyChoice(code, "Numpad $d", "Números", "$d"))
    }

    // Funciones F1-F12
    for (i in 1..12) {
        val code = KeyEvent.KEYCODE_F1 + (i - 1)
        list.add(PcKeyChoice(code, "F$i", "Funciones", "F$i"))
    }

    // Especiales y Modificadores
    list.add(PcKeyChoice(KeyEvent.KEYCODE_SPACE, "Espacio", "Especiales", "SPACE"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_ENTER, "Enter / Return", "Especiales", "ENTER"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_ESCAPE, "Escape (Esc)", "Especiales", "ESC"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_TAB, "Tabulador", "Especiales", "TAB"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DEL, "Retroceso (Backspace)", "Especiales", "BACKSPACE"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_SHIFT_LEFT, "Shift Izquierdo", "Especiales", "SHIFT"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_SHIFT_RIGHT, "Shift Derecho", "Especiales", "RSHIFT"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_CTRL_LEFT, "Control Izquierdo", "Especiales", "CTRL"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_CTRL_RIGHT, "Control Derecho", "Especiales", "RCTRL"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_ALT_LEFT, "Alt Izquierdo", "Especiales", "ALT"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_ALT_RIGHT, "Alt Derecho", "Especiales", "RALT"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_INSERT, "Insertar (Insert)", "Especiales", "INS"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_FORWARD_DEL, "Supr (Delete)", "Especiales", "DEL"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_MOVE_HOME, "Inicio (Home)", "Especiales", "HOME"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_MOVE_END, "Fin (End)", "Especiales", "END"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_PAGE_UP, "Re Pág (Page Up)", "Especiales", "PGUP"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_PAGE_DOWN, "Av Pág (Page Down)", "Especiales", "PGDN"))

    // Direcciones
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_UP, "Flecha Arriba (↑)", "Flechas", "UP"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_DOWN, "Flecha Abajo (↓)", "Flechas", "DOWN"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_LEFT, "Flecha Izquierda (←)", "Flechas", "LEFT"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_RIGHT, "Flecha Derecha (→)", "Flechas", "RIGHT"))

    // Mando y Acciones Especiales
    list.add(PcKeyChoice(96, "Mando A", "Mando", "Z"))
    list.add(PcKeyChoice(97, "Mando B", "Mando", "X"))
    list.add(PcKeyChoice(99, "Mando X", "Mando", "C"))
    list.add(PcKeyChoice(100, "Mando Y", "Mando", "V"))
    list.add(PcKeyChoice(102, "Mando L1 (LB)", "Mando", "M"))
    list.add(PcKeyChoice(103, "Mando R1 (RB)", "Mando", "B"))
    list.add(PcKeyChoice(104, "Mando L2 (LT)", "Mando", "S"))
    list.add(PcKeyChoice(105, "Mando R2 (RT)", "Mando", "O"))
    list.add(PcKeyChoice(106, "Mando Stick L3", "Mando", "E"))
    list.add(PcKeyChoice(107, "Mando Stick R3", "Mando", "E"))
    list.add(PcKeyChoice(108, "Mando Start", "Mando", "SPACE"))
    list.add(PcKeyChoice(109, "Mando Select", "Mando", "ENTER"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_UP, "D-Pad Arriba", "Mando", "UP"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_DOWN, "D-Pad Abajo", "Mando", "DOWN"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_LEFT, "D-Pad Izquierda", "Mando", "LEFT"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_DPAD_RIGHT, "D-Pad Derecha", "Mando", "RIGHT"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_MEDIA_FAST_FORWARD, "Modo Turbo", "Mando", "TURBO"))
    list.add(PcKeyChoice(KeyEvent.KEYCODE_MENU, "Rueda Radial", "Mando", "RADIAL"))

    // Ratón y PC
    list.add(PcKeyChoice(-101, "Clic Izquierdo (LMB)", "Ratón / PC", "LMB"))
    list.add(PcKeyChoice(-102, "Clic Derecho (RMB)", "Ratón / PC", "RMB"))
    list.add(PcKeyChoice(-103, "Cantidad / Scroll (+)", "Ratón / PC", "+"))
    list.add(PcKeyChoice(-104, "Cantidad / Scroll (-)", "Ratón / PC", "-"))

    list
}

/**
 * A view whose only job is to sit there focused and report every pad button or key pressed.
 */
class PadProbe(context: Context) : View(context) {
    var onKey: ((Int) -> Unit)? = null
    var onStick: ((Float, Float) -> Unit)? = null

    init {
        isFocusable = true
        isFocusableInTouchMode = true
    }

    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
        onKey?.invoke(keyCode)
        return true
    }

    override fun onGenericMotionEvent(event: MotionEvent): Boolean {
        if (event.source and InputDevice.SOURCE_JOYSTICK == InputDevice.SOURCE_JOYSTICK) {
            onStick?.invoke(event.getAxisValue(MotionEvent.AXIS_X), event.getAxisValue(MotionEvent.AXIS_Y))
            return true
        }
        return super.onGenericMotionEvent(event)
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (event.actionMasked == MotionEvent.ACTION_DOWN) requestFocus()
        return true
    }
}

fun keyChoiceForCode(code: Int): PcKeyChoice {
    return AVAILABLE_KEY_CHOICES.firstOrNull { it.code == code } ?: run {
        val tag = when (code) {
            in KeyEvent.KEYCODE_A..KeyEvent.KEYCODE_Z -> ('A'.code + (code - KeyEvent.KEYCODE_A)).toChar().toString()
            in KeyEvent.KEYCODE_0..KeyEvent.KEYCODE_9 -> (code - KeyEvent.KEYCODE_0).toString()
            in KeyEvent.KEYCODE_NUMPAD_0..KeyEvent.KEYCODE_NUMPAD_9 -> (code - KeyEvent.KEYCODE_NUMPAD_0).toString()
            in KeyEvent.KEYCODE_F1..KeyEvent.KEYCODE_F12 -> "F${code - KeyEvent.KEYCODE_F1 + 1}"
            else -> buttonName(code)
        }
        PcKeyChoice(code, buttonName(code), "Mando", tag)
    }
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
