import org.junit.Test;
import org.junit.runner.RunWith;
import org.junit.runners.Parameterized;
import java.util.Arrays;
import java.util.Collection;
import static org.junit.Assert.assertNotNull;
@RunWith(Parameterized.class)
public class UniqueParametersTest {
 @Parameterized.Parameters(name="{index}:{0}")
 public static Collection<Object[]> data() { return Arrays.asList(new Object[][]{{"same"},{"same"}}); }
 private final String value;
 public UniqueParametersTest(String value) { this.value = value; }
 @Test public void check() { assertNotNull(value); }
}
