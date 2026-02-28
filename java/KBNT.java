package frc.robot.controller;

import edu.wpi.first.networktables.IntegerArraySubscriber;
import edu.wpi.first.networktables.IntegerArrayTopic;
import edu.wpi.first.networktables.NetworkTable;
import edu.wpi.first.networktables.NetworkTableInstance;
import edu.wpi.first.networktables.StringSubscriber;
import edu.wpi.first.networktables.StringTopic;
import edu.wpi.first.wpilibj2.command.button.Trigger;
import java.util.function.BooleanSupplier;

/**
 * KBNT (Keyboard over Network Tables) is a utility that allows keyboard input to be
 * transmitted to the robot via NetworkTables.
 *
 * <p>This class interfaces with the KBNT client app over NetworkTables to read keyboard input
 * data, specifically tracking key presses and their respective press counts. It provides methods to
 * check key states and create WPILib {@link Trigger} objects for command-based programming.
 *
 * <p>The NetworkTable structure used:
 * <ul>
 *   <li>Table name: {@code "KBNT"}</li>
 *   <li>{@code "KeysToPress"} - A string of tracked keys (lowercase)</li>
 *   <li>{@code "NumKeydowns"} - An array of press counts corresponding to each key (by index)</li>
 * </ul>
 *
 * <p>Usage example:
 * <pre>
 *   KBNT kbnt = KBNT.getInstance();
 *
 *   // Check if a key is currently registered
 *   boolean isRegistered = kbnt.contains('a');
 *
 *   // Get the number of times a key has been pressed, 0 if not registered
 *   long pressCount = kbnt.count('a');
 *
 *   // Create a trigger that activates when a key is pressed. Will never trigger if not registered.
 *   Trigger aTrigger = kbnt.trigger('a');
 * </pre>
 */
public class KBNT {

    /**
     * Returns {@code true} if a new key press has been detected since the last invocation.
     * Compares the current press count from the {@link KBNT} instance against the last
     * known count. If a new press is found, updates the internal count and returns {@code true}.
     *
     * @return {@code true} if a new press of the monitored key has occurred, {@code false} otherwise
     */
    static class KBTrigger implements BooleanSupplier {

        /**
         * The {@link KBNT} instance used to query key press counts.
         */
        final KBNT kbnt;

        /**
         * The keyboard character this trigger is monitoring for presses.
         */
        final char key;

        /**
         * The number of presses that have been processed and reported by this trigger.
         * Incremented each time a new press is detected to prevent duplicate reporting.
         */
        long presses = 0;

        /**
         * Constructs a new {@code KBTrigger} that monitors the specified key on the given {@link KBNT} instance.
         *
         * @param instance the {@link KBNT} instance to query for key press counts
         * @param key      the keyboard character to monitor for presses
         */
        KBTrigger(KBNT instance, char key) {
            this.kbnt = instance;
            this.key = key;
        }

        @Override
        public boolean getAsBoolean() {
            long count = kbnt.count(key);

            if (count > presses) {
                presses = count;
                return true;
            }

            return false;
        }
    }

    static final String KEYDOWNS = "NumKeydowns";
    static final String KEYS = "KeysToPress";

    final NetworkTableInstance nt = NetworkTableInstance.getDefault();
    final NetworkTable kbnt = nt.getTable("KBNT");
    final IntegerArrayTopic keydownTopic = kbnt.getIntegerArrayTopic(KEYDOWNS);
    final StringTopic keysTopic = kbnt.getStringTopic(KEYS);

    final IntegerArraySubscriber keydowns = keydownTopic.subscribe(new long[0]);
    final StringSubscriber keys = keysTopic.subscribe("");

    private static KBNT instance;

    /** Private constructor to enforce singleton pattern. */
    private KBNT() {}

    /** Returns the singleton instance of {@code KBNT}, creating it if it does not already exist. */
    public static KBNT getInstance() {
        if (instance == null) {
            instance = new KBNT();
        }

        return instance;
    }

    /**
     * Checks if the specified key is currently registered by the KBNT client
     * @param key the keyboard character to check for registration (case-insensitive)
     * @return {@code true} if the key is registered (present in the {@link KBNT#keys} subscription), {@code false} otherwise
     */
    public boolean contains(char key) {
        String keysToPress = keys.get();
        return keysToPress.indexOf(Character.toLowerCase(key)) != -1;
    }

    /**
     * Returns the number of times the specified key has been pressed according to the KBNT client.
     * @param key the keyboard character to query press count for (case-insensitive)
     * @return the number of times the key has been pressed, or 0 if the key is not registered
     */
    public long count(char key) {
        int index = keys.get().indexOf(Character.toLowerCase(key));
        long[] counts = keydowns.get();

        if (index == -1 || index >= counts.length) {
            return 0;
        }

        return counts[index];
    }

    /**
     * Creates a WPILib {@link Trigger} that activates when the specified key is pressed between event loop runs
     * @param key the keyboard character to create a trigger for (case-insensitive)
     * @return a new {@link Trigger} instance that activates when the key is pressed
     */
    public Trigger trigger(char key) {
        return new Trigger(new KBTrigger(this, Character.toLowerCase(key)));
    }
}
