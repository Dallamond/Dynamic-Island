import { BigClock } from "../clock/Clock";
import { useActivities } from "../../core/activities";
import { useSettings } from "../../core/useSettings";
import { SystemMini } from "../system/System";

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
          {activities.map((a) => (
            <div key={a.id} className="home-activity">
              <a.Compact />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
