import org.junit.Test;
import org.junit.Ignore;
import static org.junit.Assert.assertEquals;
public class CasesTest {
 @Test public void pass() { assertEquals(4,2+2); }
 @Test public void fail() { assertEquals(5,2+2); }
 @Ignore @Test public void skip() { throw new AssertionError("ignored"); }
 @Test public void timeout() throws Exception { Thread.sleep(30000); }
}
