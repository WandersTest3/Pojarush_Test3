package com.wanderstest3.pojarush

import java.io.BufferedInputStream
import java.io.DataInputStream
import java.io.DataOutputStream
import java.io.File
import java.io.IOException
import java.util.zip.GZIPInputStream
import java.util.zip.GZIPOutputStream

sealed class Nbt {
    abstract val type: Int

    data class B(var v: Byte) : Nbt() { override val type = 1 }
    data class S(var v: Short) : Nbt() { override val type = 2 }
    data class I(var v: Int) : Nbt() { override val type = 3 }
    data class L(var v: Long) : Nbt() { override val type = 4 }
    data class F(var v: Float) : Nbt() { override val type = 5 }
    data class D(var v: Double) : Nbt() { override val type = 6 }
    class BA(var v: ByteArray) : Nbt() { override val type = 7 }
    data class Str(var v: String) : Nbt() { override val type = 8 }
    class Lst(var elem: Int, val v: MutableList<Nbt>) : Nbt() { override val type = 9 }
    class Cmp(val v: LinkedHashMap<String, Nbt>) : Nbt() { override val type = 10 }
    class IA(var v: IntArray) : Nbt() { override val type = 11 }
    class LA(var v: LongArray) : Nbt() { override val type = 12 }

    fun display(): String = when (this) {
        is B -> v.toString()
        is S -> v.toString()
        is I -> v.toString()
        is L -> v.toString()
        is F -> v.toString()
        is D -> v.toString()
        is Str -> v
        is BA -> v.joinToString(",")
        is IA -> v.joinToString(",")
        is LA -> v.joinToString(",")
        is Lst -> "${v.size}"
        is Cmp -> "${v.size}"
    }

    fun parse(text: String): Boolean = runCatching {
        val t = text.trim()
        fun parts() = if (t.isEmpty()) emptyList() else t.split(',').map { it.trim() }
        when (this) {
            is B -> v = t.toByte()
            is S -> v = t.toShort()
            is I -> v = t.toInt()
            is L -> v = t.toLong()
            is F -> v = t.toFloat()
            is D -> v = t.toDouble()
            is Str -> v = text
            is BA -> v = parts().map { it.toByte() }.toByteArray()
            is IA -> v = parts().map { it.toInt() }.toIntArray()
            is LA -> v = parts().map { it.toLong() }.toLongArray()
            else -> error("container")
        }
    }.isSuccess

    companion object {
        val NAMES = arrayOf("End", "Byte", "Short", "Int", "Long", "Float", "Double", "Byte[]", "String", "List", "Compound", "Int[]", "Long[]")

        private fun read(t: Int, d: DataInputStream, depth: Int): Nbt {
            if (depth > 512) throw IOException("NBT too deep")
            return when (t) {
                1 -> B(d.readByte())
                2 -> S(d.readShort())
                3 -> I(d.readInt())
                4 -> L(d.readLong())
                5 -> F(d.readFloat())
                6 -> D(d.readDouble())
                7 -> BA(ByteArray(d.readInt()).also { d.readFully(it) })
                8 -> Str(d.readUTF())
                9 -> {
                    val e = d.readByte().toInt()
                    val n = d.readInt()
                    Lst(e, MutableList(n.coerceAtLeast(0)) { read(e, d, depth + 1) })
                }
                10 -> {
                    val m = LinkedHashMap<String, Nbt>()
                    while (true) {
                        val k = d.readByte().toInt()
                        if (k == 0) break
                        val name = d.readUTF()
                        m[name] = read(k, d, depth + 1)
                    }
                    Cmp(m)
                }
                11 -> IA(IntArray(d.readInt()) { d.readInt() })
                12 -> LA(LongArray(d.readInt()) { d.readLong() })
                else -> throw IOException("Unknown NBT tag $t")
            }
        }

        private fun write(n: Nbt, d: DataOutputStream) {
            when (n) {
                is B -> d.writeByte(n.v.toInt())
                is S -> d.writeShort(n.v.toInt())
                is I -> d.writeInt(n.v)
                is L -> d.writeLong(n.v)
                is F -> d.writeFloat(n.v)
                is D -> d.writeDouble(n.v)
                is BA -> { d.writeInt(n.v.size); d.write(n.v) }
                is Str -> d.writeUTF(n.v)
                is Lst -> {
                    d.writeByte(if (n.v.isEmpty()) n.elem else n.v[0].type)
                    d.writeInt(n.v.size)
                    n.v.forEach { write(it, d) }
                }
                is Cmp -> {
                    n.v.forEach { (k, v) ->
                        d.writeByte(v.type)
                        d.writeUTF(k)
                        write(v, d)
                    }
                    d.writeByte(0)
                }
                is IA -> { d.writeInt(n.v.size); n.v.forEach(d::writeInt) }
                is LA -> { d.writeInt(n.v.size); n.v.forEach(d::writeLong) }
            }
        }
    }

    class Document(val file: File, val compressed: Boolean, val rootName: String, val root: Nbt) {
        fun save() {
            val backup = File(file.parentFile, file.name + ".bak")
            if (!backup.exists()) file.copyTo(backup)
            val tmp = File(file.parentFile, file.name + ".tmp")
            tmp.outputStream().buffered().use { raw ->
                val out = if (compressed) GZIPOutputStream(raw) else raw
                DataOutputStream(out).use { d ->
                    d.writeByte(root.type)
                    d.writeUTF(rootName)
                    write(root, d)
                }
            }
            if (!tmp.renameTo(file)) {
                tmp.copyTo(file, true)
                tmp.delete()
            }
        }

        companion object {
            fun open(file: File): Document {
                val head = file.inputStream().use { val b = ByteArray(2); it.read(b); b }
                val gz = head[0] == 0x1f.toByte() && head[1] == 0x8b.toByte()
                BufferedInputStream(file.inputStream()).use { raw ->
                    val input = if (gz) GZIPInputStream(raw) else raw
                    val d = DataInputStream(input)
                    val t = d.readByte().toInt()
                    if (t != 10) throw IOException("Not an NBT file")
                    val name = d.readUTF()
                    return Document(file, gz, name, read(t, d, 0))
                }
            }
        }
    }
}
