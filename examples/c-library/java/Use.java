// examples/c-library/java/Use.java - a Java program using the library through
// the binding `nikaia bind java` writes (ADR-284 D34); it prints what `use.c`
// prints. Java 22 or later:
//
//   nikaia bind java
//   javac --release 22 -d target/java target/nikaia/c-library/java/wordtally/Wordtally.java java/Use.java
//   LD_LIBRARY_PATH=target/nikaia/c-library java --enable-native-access=ALL-UNNAMED -cp target/java Use
import java.util.Locale;
import java.util.concurrent.atomic.AtomicInteger;

import wordtally.Wordtally;

public final class Use {
    private Use() {}

    public static void main(String[] args) {
        String[] lines = {"the quick brown fox", "jumps over the lazy dog"};
        try (Wordtally.Tally tally = new Wordtally.Tally()) {
            long words = 0;
            for (String line : lines) {
                words = tally.add(line);
            }
            // An error status is an exception (D34), so a status printed here is 0.
            System.out.printf("%d words, the longest \"%s\" (status %d)%n", words, tally.longest(), 0);
        }

        String shout = "Hello NIKAIA from c";
        AtomicInteger left = new AtomicInteger(2);
        System.out.printf("the first two words of \"%s\":%n", shout);
        long seen = Wordtally.eachWord(shout, (word, wordCase) -> {
            System.out.printf("  %s (%s)%n", word, wordCase.name().toLowerCase(Locale.ROOT));
            return left.decrementAndGet() > 0;
        });
        System.out.printf("stopped after %d%n", seen);
    }
}
