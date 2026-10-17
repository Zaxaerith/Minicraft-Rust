// GPL-3.0-only. Read-only constructor probes; no replaced RNG or game algorithm.
package minicraft.level;

import java.lang.instrument.*;
import java.security.ProtectionDomain;
import jdk.internal.org.objectweb.asm.*;

public final class WorldProbe36 {
    public static void premain(String args, Instrumentation instrumentation) {
        instrumentation.addTransformer(new ClassFileTransformer() {
            public byte[] transform(ClassLoader loader, String name, Class<?> type,
                ProtectionDomain domain, byte[] bytes) {
                if (!name.equals("minicraft/level/Level")) return null;
                ClassReader reader = new ClassReader(bytes);
                ClassWriter writer = new ClassWriter(reader, ClassWriter.COMPUTE_MAXS);
                reader.accept(new ClassVisitor(Opcodes.ASM8, writer) {
                    public MethodVisitor visitMethod(int access, String method, String descriptor,
                        String signature, String[] exceptions) {
                        MethodVisitor target = super.visitMethod(access,method,descriptor,signature,exceptions);
                        if (!method.equals("<init>") || !descriptor.equals("(IIJILminicraft/level/Level;Z)V")) return target;
                        return new MethodVisitor(Opcodes.ASM8,target) {
                            void observe(String stage) {
                                super.visitVarInsn(Opcodes.ALOAD,0);
                                super.visitLdcInsn(stage);
                                super.visitMethodInsn(Opcodes.INVOKESTATIC,"minicraft/level/WorldStages36","observe",
                                    "(Lminicraft/level/Level;Ljava/lang/String;)V",false);
                            }
                            public void visitMethodInsn(int opcode,String owner,String method,String desc,boolean itf) {
                                if (owner.equals("minicraft/level/Level") && method.equals("checkChestCount")) observe("connections");
                                super.visitMethodInsn(opcode,owner,method,desc,itf);
                                if (owner.equals("minicraft/level/LevelGen") && method.equals("createAndValidateMap")) {
                                    // Returned ChunkManager is still on the operand stack.
                                    // Pass it separately, without touching the following PUTFIELD.
                                    super.visitInsn(Opcodes.DUP);
                                    super.visitVarInsn(Opcodes.ALOAD,0);
                                    super.visitMethodInsn(Opcodes.INVOKESTATIC,"minicraft/level/WorldStages36","raw",
                                        "(Lminicraft/level/ChunkManager;Lminicraft/level/Level;)V",false);
                                }
                                if (owner.equals("minicraft/level/Level")) {
                                    if (method.equals("generateSpawnerStructures")) observe("spawners");
                                    if (method.equals("generateVillages")) observe("villages");
                                    if (method.equals("generateDungeonStructures")) observe("dungeons");
                                    if (method.equals("checkChestCount")) observe("chests");
                                    if (method.equals("checkAirWizard")) observe("final");
                                }
                            }
                        };
                    }
                },0);
                return writer.toByteArray();
            }
        });
    }
}
