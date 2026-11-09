/** 房间名/昵称随机生成器：中文词库（形容词 + 名词意象） */

const ADJECTIVES = [
  '晴空', '微风', '星河', '落日', '晨露', '晚风', '青竹', '流云',
  '碧海', '暖阳', '疏影', '月色', '山间', '清泉', '灯火', '长风',
]

const NOUNS = [
  '茶馆', '鹿鸣', '小栈', '星辰', '白鹭', '青柠', '海豚', '松鼠',
  '萤火', '竹林', '灯塔', '小溪', '木屋', '桔梗', '麦田', '云朵',
]

function pickRandom<T>(list: T[]): T {
  return list[Math.floor(Math.random() * list.length)]
}

export function randomRoomName(): string {
  return `${pickRandom(ADJECTIVES)}${pickRandom(NOUNS)}`
}

export function randomNickname(): string {
  return `${pickRandom(ADJECTIVES)}的${pickRandom(NOUNS)}`
}
