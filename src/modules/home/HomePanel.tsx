import { BigClock } from "../clock/Clock";
import { useActivities } from "../../core/activities";
import { useSettings } from "../../core/useSettings";
import { SystemMini } from "../system/System";

/** Actividades que caben bajo el reloj; el resto se resume en "+N". */
const MAX_HOME_ACTIVITIES = 2;

/** Vista de inicio: reloj grande, CPU/RAM/GPU básicos y las actividades en curso. */
export function HomePanel() {
  const activities = useActivities();
  const settings = useSettings();
  return (
    <div className="home">
      <BigClock />
      {settings?.modules.system && <SystemMini />}
      {activities.length > 0 && (
        <div className="home-activities">
          {activities.slice(0, MAX_HOME_ACTIVITIES).map((a) => (
            <div key={a.id} className="home-activity">
              <a.Compact />
            </div>
          ))}
          {activities.length > MAX_HOME_ACTIVITIES && (
            <div className="home-activity muted">+{activities.length - MAX_HOME_ACTIVITIES}</div>
          )}
        </div>
      )}
    </div>
  );
}
