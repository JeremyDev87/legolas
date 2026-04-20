import { format } from "date-fns";
import { fr } from "date-fns/locale/fr";
import { ko } from "date-fns/locale/ko";
import dayjs from "dayjs";
import "dayjs/locale/ja";
import "dayjs/locale/ko";
import moment from "moment";
import "moment/locale/fr";
import "moment/locale/ko";

export function DateView() {
  const today = new Date("2026-04-21T00:00:00.000Z");

  return (
    <time dateTime={today.toISOString()}>
      {format(today, "PPP", { locale: ko })}
      {format(today, "PPP", { locale: fr })}
      {dayjs(today).locale("ko").format("YYYY-MM-DD")}
      {moment(today).locale("fr").format("YYYY-MM-DD")}
    </time>
  );
}
