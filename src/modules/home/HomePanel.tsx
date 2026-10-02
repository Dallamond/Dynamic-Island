import { BigClock } from "../clock/Clock";
import { useActivities } from "../../core/activities";

/** Vista de inicio: reloj grande y, debajo, las actividades en curso. */
export function HomePanel() {
  const activities = useActivities();
  return (
    <div className="home">
      <BigClock />
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
