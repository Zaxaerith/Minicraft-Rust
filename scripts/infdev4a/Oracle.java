package infdev4a;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import minicraft.level.noise.LevelNoise;

/** Runs the original pinned noise, without changing its seed or algorithms. */
public final class Oracle {
    public static void main(String[] args) throws Exception {
        int size = Integer.parseInt(args[1]);
        long[] seeds = {0, 42, -1, Long.MIN_VALUE, Long.MAX_VALUE};
        int[][] points = {{0,0},{63,0},{64,0},{-1,-1},{-64,-64},{-65,63},
            {1000000,-1000000},{Integer.MIN_VALUE,0},
            {Integer.MAX_VALUE,Integer.MAX_VALUE},{Integer.MIN_VALUE,Integer.MAX_VALUE}};
        StringBuilder out = new StringBuilder("# Original Infdev 42ba9192fcc229bcf615453a6504d415c777feaf; seed x y chunkX chunkY localIndex heightBits\n");
        for (long seed : seeds) for (int[] p : points) {
            LevelNoise noise = new LevelNoise(seed,p[0],p[1],0,1,1);
            out.append(seed).append('\t').append(p[0]).append('\t').append(p[1]).append('\t')
               .append(Math.floorDiv(p[0],size)).append('\t').append(Math.floorDiv(p[1],size)).append('\t')
               .append(Math.floorMod(p[0],size)+Math.floorMod(p[1],size)*size).append('\t')
               .append(String.format("%016x",Double.doubleToLongBits(noise.getHeight(0,0)))).append('\n');
        }
        Files.writeString(Path.of(args[0]),out,StandardCharsets.UTF_8);
        System.out.println("PASS original LevelNoise: 50 signed-seed/global-coordinate samples; chunk size "+size);
    }
}
