use std::ops::{ Index, IndexMut };

#[derive(Debug, Default, Clone)]
pub struct Vec2d<T> {
    data: Vec<T>,

    _width: usize,
    _height: usize,
}

impl<T> Vec2d<T> {
    fn with_capacity(width: usize, height: usize) -> Vec<T> {
        Vec::with_capacity(Self::calc_size(width, height))
    }

    fn calc_size(width: usize, height: usize) -> usize {
        width * height
    }

    pub fn width(&self) -> usize {
        self._width
    }

    pub fn height(&self) -> usize {
        self._height
    }

    pub fn size(&self) -> usize { 
        self._width * self._height
    }

    fn get_index(&self, x: usize, y: usize) -> usize {
        (y * self._width) + x
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut T> {
        let idx = self.get_index(x, y);
        self.data.get_mut(idx)
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&T> {
        let idx = self.get_index(x, y);
        self.data.get(idx)
    }

    pub fn insert(&mut self, x: usize, y: usize, val: T) -> Option<&mut T> {
        let idx = self.get_index(x, y);
        let elem = self.data.get_mut(idx)?;

        *elem = val;
        Some(elem)
    }

    pub fn iter(&self) -> std::slice::Iter<T> {
        self.data.iter()
    }
}

impl<T> Vec2d<T> where T: Default {
    pub fn new(width: usize, height: usize) -> Self {
        let size = Self::calc_size(width, height);
        let mut data = Self::with_capacity(width, height);
        for _ in 0..size {
            data.push(T::default());
        }

        Self { data, _width: width, _height: height }
    }
}

impl<T> Vec2d<T> where T: Clone {
    pub fn from_slice(width: usize, height: usize, slice: &[T]) -> Self {
        let size = Self::calc_size(width, height);
        let mut data = Self::with_capacity(width, height);
        for i in 0..size {
            data.push(slice[i].clone());
        }

        Self { data, _width: width, _height: height }
    }

}

impl<T> Vec2d<T> where T: Copy {
    pub fn new_with(width: usize, height: usize, val: T) -> Self {
        let data = vec![val; Self::calc_size(width, height)];
        Self { data, _width: width, _height: height }
    }
}

impl<T> PartialEq for Vec2d<T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        let same_elems = self.data == other.data;
        let same_width = self.width() == other.width();
        let same_height = self.height() == other.height();

        same_elems && same_width && same_height
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

impl<T> Into<Vec<T>> for Vec2d<T> {
    fn into(self) -> Vec<T> {
        self.data
    }
}

impl<T> Index<usize> for Vec2d<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<T> IndexMut<usize> for Vec2d<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}
