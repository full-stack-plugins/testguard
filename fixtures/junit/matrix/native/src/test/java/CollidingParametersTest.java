import org.junit.Test;
import org.junit.runner.RunWith;
import org.junit.runners.Parameterized;
import java.util.Arrays;
import java.util.Collection;
import static org.junit.Assert.assertNotNull;
@RunWith(Parameterized.class)
public class CollidingParametersTest {
 @Parameterized.Parameters(name="{0}")
 public static Collection<Object[]> data() { return Arrays.asList(new Object[][]{{"same"},{"same"}}); }
 private final String value;
 public CollidingParametersTest(String value) { this.value = value; }
 @Test public void check() { assertNotNull(value); }
}
