package frc.robot.controller;

import edu.wpi.first.wpilibj2.command.button.CommandXboxController;
import edu.wpi.first.wpilibj2.command.button.Trigger;

public class XboxEliteController extends CommandXboxController {

    /**
     * Paddle triggers in the following order: [BL, TL, TR, BR].
     * <br> They will read left-to-right in the Xbox Accessories app.
     */
    final Trigger[] paddles = new Trigger[4];

    /**
     * Creates an XboxEliteController instance with paddle triggers mapped to the specified keys.
     *
     * @param port The USB port number for the controller (e.g., 0 for the first controller).
     * @param paddleKeys The keys assigned to the paddles in order [BL, TL, TR, BR]. For example, "team" would assign 't' to BL, 'e' to TL, and so on.
     */
    public XboxEliteController(int port, String paddleKeys) {
        super(port);
        KBNT kbnt = KBNT.getInstance();
        for (int i = 0; i < 4; i++) {
            paddles[i] = kbnt.trigger(paddleKeys.charAt(i));
        }
    }

    public Trigger paddleBL() {
        return paddles[0];
    }

    public Trigger paddleTL() {
        return paddles[1];
    }

    public Trigger paddleTR() {
        return paddles[2];
    }

    public Trigger paddleBR() {
        return paddles[3];
    }
}
