import org.junit.Test;
import org.junit.Ignore;
import static org.junit.Assert.*;
/** Fixed local fixture only; not candidate test code. */
public class CasesTest {
 @Test public void passA() { assertEquals(4, 2 + 2); }
 @Test public void passB() { assertTrue(true); }
 @Test public void fail() { assertEquals(5, 2 + 2); }
 @Ignore("fixed skip fixture") @Test public void skip() { org.junit.Assert.fail("must not execute"); }
 @Test public void timeout() throws Exception {
   System.out.println("TG_TIMEOUT_READY"); System.out.flush();
   Thread.sleep(30000);
 }
}
